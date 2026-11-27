//! The `check` tool as a task: a bundle with at least [`CHECK_TASK_THRESHOLD`]
//! records in force is checked in the background and polled, for a client that
//! declared the tasks extension; every other call answers inline.

use crate::args::CheckArgs;
use crate::BundleServer;
use fs_store::FsStore;
use living_docs_core::commands::in_force::in_force_records;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolRequestParams, CreateTaskResult, GetTaskResult};
use rmcp::task_manager::{TaskExit, TaskManager, TaskOptions};
use rmcp::ErrorData;
use serde_json::Value;
use std::path::Path;

/// The count of records in force from which `check` runs as a task. Measured
/// where a check takes two seconds, the default polling interval of Claude
/// Code; the host and the curve are in `scripts/bench/check-task-threshold.md`.
pub(crate) const CHECK_TASK_THRESHOLD: usize = 2670;

/// The polling interval a task advertises, the one the threshold was measured
/// against.
const POLL_INTERVAL_MS: u64 = 2000;

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

    /// Starts the check on a blocking thread. A cancelled task ends at once;
    /// the check itself runs to its end and its report is dropped.
    pub(crate) fn spawn(&self, server: BundleServer, args: CheckArgs) -> CreateTaskResult {
        let options = TaskOptions::default().with_poll_interval_ms(POLL_INTERVAL_MS);
        let task = self.manager.spawn(options, move |context| {
            Box::pin(async move {
                let check = tokio::task::spawn_blocking(move || server.check(Parameters(args)));
                tokio::select! {
                    () = context.cancelled() => Err(TaskExit::Cancelled),
                    done = check => done.map_err(|error| {
                        TaskExit::Error(ErrorData::internal_error(error.to_string(), None))
                    }),
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
