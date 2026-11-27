//! The `check` tool as a task: a bundle with at least [`CHECK_TASK_THRESHOLD`]
//! records in force is checked in the background and polled, for a client that
//! declared the tasks extension; every other call answers inline.

use crate::args::CheckArgs;
use crate::BundleServer;
use fs_store::FsStore;
use crate::tools::shaped;
use living_docs_core::check::{compile_cancellable, Checked};
use living_docs_core::commands::in_force::in_force_records;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolRequestParams, CreateTaskResult, GetTaskResult};
use rmcp::task_manager::{TaskExit, TaskManager, TaskOptions};
use rmcp::ErrorData;
use serde_json::Value;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The count of records in force from which `check` runs as a task. Measured
/// where a check takes two seconds, the default polling interval of Claude
/// Code; the host and the curve are in `scripts/bench/check-task-threshold.md`.
pub(crate) const CHECK_TASK_THRESHOLD: usize = 2670;

/// The polling interval a task advertises, the one the threshold was measured
/// against.
const POLL_INTERVAL_MS: u64 = 2000;

/// What a check run under a cancel flag yields: the tool result, or nothing
/// once the flag stopped it.
pub(crate) enum TaskCheck {
    Report(rmcp::model::CallToolResult),
    Cancelled,
}

/// Runs `check` for `server` with `args`, stopping between its passes once
/// `cancel` is raised.
pub(crate) fn run_cancellable_check(
    server: &BundleServer,
    args: CheckArgs,
    cancel: &AtomicBool,
) -> TaskCheck {
    let docs_dir = &server.docs_dir;
    let store = FsStore::rooted(docs_dir);
    match compile_cancellable(&store, docs_dir, args.require_owner, cancel) {
        Some(Checked::Report(report)) => TaskCheck::Report(shaped(
            serde_json::to_value(&report).map_err(|error| error.to_string()),
        )),
        Some(Checked::Cancelled) => TaskCheck::Cancelled,
        None => TaskCheck::Report(server.check(Parameters(args))),
    }
}

fn join_failure(error: tokio::task::JoinError) -> Result<rmcp::model::CallToolResult, TaskExit> {
    Err(TaskExit::Error(ErrorData::internal_error(
        error.to_string(),
        None,
    )))
}

/// The tasks of one server and the record count that turns a `check` call
/// into one.
#[derive(Clone)]
pub(crate) struct CheckTasks {
    manager: TaskManager,
    threshold: usize,
}

impl CheckTasks {
    pub(crate) fn new(threshold: usize) -> Self {
        Self {
            manager: TaskManager::new(),
            threshold,
        }
    }

    /// The arguments of a `check` call that must run as a task: the client
    /// declared the extension and the bundle holds enough records. Counted
    /// before any check runs; arguments that do not parse answer inline, where
    /// the tool reports them.
    pub(crate) fn wanted(
        &self,
        docs_dir: &Path,
        request: &CallToolRequestParams,
        client_declared_tasks: bool,
    ) -> Option<CheckArgs> {
        if !client_declared_tasks || request.name != "check" {
            return None;
        }
        let arguments = Value::Object(request.arguments.clone().unwrap_or_default());
        let args: CheckArgs = serde_json::from_value(arguments).ok()?;
        let records = in_force_records(&FsStore::rooted(docs_dir), docs_dir).len();
        (records >= self.threshold).then_some(args)
    }

    /// Starts the check on a blocking thread. Cancelling the task raises a
    /// flag the check looks at between its passes; the task ends cancelled
    /// once the thread has stopped, with no report.
    pub(crate) fn spawn(&self, server: BundleServer, args: CheckArgs) -> CreateTaskResult {
        let options = TaskOptions::default().with_poll_interval_ms(POLL_INTERVAL_MS);
        let task = self.manager.spawn(options, move |context| {
            Box::pin(async move {
                let cancel = Arc::new(AtomicBool::new(false));
                let flag = Arc::clone(&cancel);
                let check = tokio::task::spawn_blocking(move || {
                    run_cancellable_check(&server, args, &flag)
                });
                tokio::pin!(check);
                let done = tokio::select! {
                    () = context.cancelled() => {
                        cancel.store(true, Ordering::SeqCst);
                        return check.await.map_or_else(join_failure, |_| Err(TaskExit::Cancelled));
                    }
                    done = &mut check => done,
                };
                match done {
                    Ok(TaskCheck::Report(result)) => Ok(result),
                    Ok(TaskCheck::Cancelled) => Err(TaskExit::Cancelled),
                    Err(error) => join_failure(error),
                }
            })
        });
        CreateTaskResult::new(task)
    }

    pub(crate) fn get(&self, task_id: &str) -> Result<GetTaskResult, ErrorData> {
        self.manager.get_task(task_id).map(GetTaskResult::new)
    }

    pub(crate) fn cancel(&self, task_id: &str) -> Result<(), ErrorData> {
        self.manager.cancel_task(task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolResult;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Bundle(std::path::PathBuf);

    impl Bundle {
        fn new(label: &str, files: &[(&str, &str)]) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock after the epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("living-docs-task-{label}-{nanos}"));
            std::fs::create_dir_all(&root).expect("create scratch bundle");
            for (name, text) in files {
                std::fs::write(root.join(name), text).expect("write scratch file");
            }
            Self(root)
        }
    }

    impl Drop for Bundle {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn args() -> CheckArgs {
        CheckArgs {
            require_owner: false,
        }
    }

    fn json(result: &CallToolResult) -> Value {
        serde_json::to_value(result).expect("result serializes")
    }

    #[test]
    fn a_flag_raised_before_the_run_cancels_without_a_report() {
        let bundle = Bundle::new("raised", &[("index.md", "# Index\n")]);
        let server = BundleServer::new(&bundle.0);
        let flag = AtomicBool::new(true);

        let outcome = run_cancellable_check(&server, args(), &flag);

        assert!(matches!(outcome, TaskCheck::Cancelled));
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn a_flag_never_raised_returns_the_inline_result() {
        let with_violation = Bundle::new("bad", &[]);
        let clean = Bundle::new("good", &[("index.md", "# Index\n")]);
        for bundle in [with_violation, clean] {
            let server = BundleServer::new(&bundle.0);
            let flag = AtomicBool::new(false);

            let outcome = run_cancellable_check(&server, args(), &flag);

            let TaskCheck::Report(result) = outcome else {
                panic!("a run nothing cancelled must return a result");
            };
            assert_eq!(json(&result), json(&server.check(Parameters(args()))));
        }
    }

    #[test]
    fn a_missing_bundle_returns_the_inline_tool_error() {
        let missing = std::env::temp_dir().join("living-docs-task-absent-bundle");
        let server = BundleServer::new(&missing);
        let flag = AtomicBool::new(false);

        let outcome = run_cancellable_check(&server, args(), &flag);

        let TaskCheck::Report(result) = outcome else {
            panic!("a missing bundle must return the tool error");
        };
        assert_eq!(result.is_error, Some(true));
        assert!(json(&result).to_string().contains("bundle root not found"));
    }
}

