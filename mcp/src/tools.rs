use crate::args::{
    CheckArgs, ManageArgs, NewArgs, ReadArgs, RecordsArgs, SetArgs, SupersedeArgs, WriteArgs,
};
use crate::BundleServer;
use fs_store::{git, FsStore};
use living_docs_core::check;
use living_docs_core::commands::effective::{self, Dependents};
use living_docs_core::commands::fmt::{self, Outcome as FmtOutcome, Seeds};
use living_docs_core::commands::index::Indexed;
use living_docs_core::commands::{index, new, set, supersede};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool, tool_router};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

type Answer = Result<Value, String>;
type Written = Result<Value, Refusal>;

fn store(docs_dir: &Path) -> FsStore {
    FsStore::rooted(docs_dir)
}

fn shown(path: &Path) -> String {
    path.display().to_string()
}

fn shown_all(paths: &[PathBuf]) -> Vec<String> {
    paths.iter().map(|path| shown(path)).collect()
}

/// Why a call is an error result: a plain message, or a write that landed but
/// whose index refresh failed, so the result still names what was written.
enum Refusal {
    Message(String),
    Written(Value),
}

impl From<String> for Refusal {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

/// Adds the written indexes to a write's result, as the CLI prints them; when
/// the refresh failed the result also carries the failures and becomes an error.
fn indexed(mut written: Value, done: &Indexed) -> Result<Value, Refusal> {
    written["indexed"] = json!(shown_all(&done.written));
    if done.failures.is_empty() {
        return Ok(written);
    }
    let failures: Vec<String> = done.failures.iter().map(ToString::to_string).collect();
    written["failures"] = json!(failures);
    Err(Refusal::Written(written))
}

fn run_new(docs_dir: &Path, args: &NewArgs) -> Written {
    let seeded = git::seeded_owner(docs_dir, &args.doc_type, args.owner.as_deref());
    let opts = new::NewOptions {
        description: args.description.as_deref(),
        kind: args.kind.as_deref(),
        owner: args.owner.as_deref().or(seeded.as_deref()),
        body: args.body.as_deref(),
    };
    let created = new::write(
        &store(docs_dir),
        docs_dir,
        &args.doc_type,
        &args.title,
        &opts,
    )?;
    let written = json!({
        "path": shown(&created.path),
        "instruction": new::BODY_ONLY_INSTRUCTION,
    });
    indexed(written, &created.indexed)
}

fn run_set(docs_dir: &Path, args: &SetArgs) -> Written {
    let store = store(docs_dir);
    let reference = args.reference.as_str();
    let (key, value, applied) = match (&args.key, &args.value, &args.heading, &args.text) {
        (Some(key), Some(value), None, None) => {
            let applied = set::apply(&store, docs_dir, reference, key, value)?;
            (key.as_str(), value.as_str(), applied)
        }
        (None, None, Some(heading), Some(text)) => {
            let applied = set::apply_section(&store, docs_dir, reference, heading, text)?;
            ("section", heading.as_str(), applied)
        }
        _ => {
            return Err("set takes `key` and `value`, or `heading` and `text`"
                .to_string()
                .into())
        }
    };
    let written = json!({
        "path": shown(&applied.path),
        "key": key,
        "value": value,
        "rewritten": shown_all(&applied.rewritten),
    });
    indexed(written, &applied.indexed)
}

fn run_supersede(docs_dir: &Path, args: &SupersedeArgs) -> Written {
    let done = supersede::supersede(&store(docs_dir), docs_dir, &args.old, &args.new)?;
    let written = json!({
        "old": shown(&done.old),
        "new": shown(&done.new),
    });
    indexed(written, &done.indexed)
}

fn run_write(docs_dir: &Path, args: &WriteArgs) -> Written {
    match args {
        WriteArgs::New(args) => run_new(docs_dir, args),
        WriteArgs::Set(args) => run_set(docs_dir, args),
        WriteArgs::Supersede(args) => run_supersede(docs_dir, args),
    }
}

fn dependents_of(docs_dir: &Path, args: &RecordsArgs) -> Result<Option<Dependents>, String> {
    let conflicts = [
        args.topic.is_some() && (args.links_to.is_some() || args.impact.is_some()),
        args.full && args.contract,
    ];
    if conflicts.contains(&true) {
        return Err(
            "topic, links_to and impact exclude one another, as do full and contract".to_string(),
        );
    }
    effective::resolve_dependents(
        &store(docs_dir),
        docs_dir,
        args.links_to.as_deref(),
        args.impact.as_deref(),
    )
}

fn run_read(docs_dir: &Path, args: &ReadArgs) -> Answer {
    let ReadArgs::Records(args) = args;
    let options = effective::Options {
        topic: args.topic.clone(),
        full: args.full,
        contract: args.contract,
        dependents: dependents_of(docs_dir, args)?,
    };
    let reading = effective::read(&store(docs_dir), docs_dir, &options, true);
    serde_json::from_str(&reading.output).map_err(|error| error.to_string())
}

fn run_manage(docs_dir: &Path, args: &ManageArgs) -> Answer {
    let store = store(docs_dir);
    match args {
        ManageArgs::Index(args) => {
            let written = index::write(&store, docs_dir, args.doc_type.clone())?;
            Ok(json!({ "files": shown_all(&written) }))
        }
        ManageArgs::Fmt(args) => {
            let seeds = Seeds {
                owner: git::identity(docs_dir),
                added_at: Some(git::added_at),
            };
            let outcome = fmt::outcome(&store, docs_dir, args.check_only, &seeds)?;
            Ok(match outcome {
                FmtOutcome::Rewritten(paths) => json!({ "files": shown_all(&paths) }),
                FmtOutcome::Pending(paths) => json!({ "pending": shown_all(&paths) }),
            })
        }
    }
}

fn run_check(docs_dir: &Path, args: &CheckArgs) -> Answer {
    let report = check::compile(&store(docs_dir), docs_dir, args.require_owner)
        .ok_or_else(|| format!("bundle root not found: {}", docs_dir.display()))?;
    serde_json::to_value(&report).map_err(|error| error.to_string())
}

fn shaped<E: Into<Refusal>>(answer: Result<Value, E>) -> CallToolResult {
    match answer.map_err(Into::into) {
        Ok(data) => CallToolResult::structured(data),
        Err(Refusal::Written(data)) => CallToolResult::structured_error(data),
        Err(Refusal::Message(message)) => CallToolResult::error(vec![ContentBlock::text(message)]),
    }
}

#[tool_router(router = tool_router, vis = "pub(crate)")]
impl BundleServer {
    #[tool(
        name = "write",
        description = "Create or change a record. operation `new` scaffolds a record (type, title, optional description, kind, owner, body); `set` changes one field (reference, key, value) or replaces one body section (reference, heading, text); `supersede` retires `old` in favor of `new`."
    )]
    pub(crate) fn write(&self, Parameters(args): Parameters<WriteArgs>) -> CallToolResult {
        shaped(run_write(&self.docs_dir, &args))
    }

    #[tool(
        name = "read",
        description = "Read the bundle. operation `records` returns the in-force records, filtered by `topic`, or by `links_to` / `impact` for the records that depend on one."
    )]
    pub(crate) fn read(&self, Parameters(args): Parameters<ReadArgs>) -> CallToolResult {
        shaped(run_read(&self.docs_dir, &args))
    }

    #[tool(
        name = "manage",
        description = "Maintain the bundle. operation `index` regenerates indexes; `fmt` canonicalizes frontmatter (check_only reports instead of writing)."
    )]
    pub(crate) fn manage(&self, Parameters(args): Parameters<ManageArgs>) -> CallToolResult {
        shaped(run_manage(&self.docs_dir, &args))
    }

    #[tool(
        name = "check",
        description = "Run the gate over the bundle. The report (ok, violations, advisories) is the result, also when the gate fails."
    )]
    pub(crate) fn check(&self, Parameters(args): Parameters<CheckArgs>) -> CallToolResult {
        shaped(run_check(&self.docs_dir, &args))
    }
}
