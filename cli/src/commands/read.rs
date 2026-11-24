//! `read` verb wrapper: delegates to `living_docs_core::commands::effective`
//! to compile and print the agent-facing view, as colored text or JSON per the
//! resolved output mode, and explains an empty `--topic` result on stderr.

use crate::args::ReadArgs;
use crate::output::{write_stdout_bytes, OutputMode};
use crate::store::build_store;
use living_docs_core::commands::effective::{self, empty_topic_note, Options, Reading};
use serde_json::Value;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_read(docs_dir: &Path, args: ReadArgs, mode: OutputMode, quiet: bool) -> ExitCode {
    let store = build_store(docs_dir);
    let dependents = match effective::resolve_dependents(
        store.as_ref(),
        docs_dir,
        args.links_to.as_deref(),
        args.impact.as_deref(),
    ) {
        Ok(dependents) => dependents,
        Err(message) => {
            eprintln!("living-docs read records: {message}");
            return ExitCode::from(2);
        }
    };
    let options = Options {
        topic: args.topic,
        full: args.full,
        contract: args.contract,
        all: args.all,
        dependents,
    };
    let reading = effective::read(store.as_ref(), docs_dir, &options, mode.is_json());
    match options.topic.as_deref().filter(|_| reading.matched == 0) {
        Some(topic) => {
            let note = empty_topic_note(reading.in_force, topic, docs_dir);
            report_empty(&reading, &note, topic, options.all, (mode, quiet));
        }
        None => write_stdout_bytes(reading.output.as_bytes()),
    }
    ExitCode::SUCCESS
}

/// Prints an empty topic answer: stdout as usual, with `in_force` and `note`
/// added in JSON, and the explanation on stderr unless `quiet`.
fn report_empty(reading: &Reading, note: &str, topic: &str, all: bool, how: (OutputMode, bool)) {
    let (mode, quiet) = how;
    let output = if mode.is_json() {
        with_note(reading, note)
    } else {
        reading.output.clone()
    };
    write_stdout_bytes(output.as_bytes());
    if !quiet {
        let live = if all { "" } else { "live " };
        eprintln!(
            "living-docs read records: no {live}record mentions '{topic}' ({} retired withheld); {note}",
            reading.withheld
        );
    }
}

/// Adds `in_force` and `note` to the empty JSON answer, keeping its own
/// fields; an output that does not parse is returned as it is.
fn with_note(reading: &Reading, note: &str) -> String {
    let Ok(Value::Object(mut fields)) = serde_json::from_str(&reading.output) else {
        return reading.output.clone();
    };
    fields.insert("in_force".to_string(), reading.in_force.into());
    fields.insert("note".to_string(), note.into());
    Value::Object(fields).to_string()
}
