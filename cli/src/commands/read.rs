//! `read` verb wrapper: delegates to `living_docs_core::commands::effective`
//! to compile and print the agent-facing view, as colored text or JSON per the
//! resolved output mode, and explains an empty `--topic` result on stderr.

use crate::args::ReadArgs;
use crate::output::{write_stdout_bytes, OutputMode};
use crate::store::build_store;
use living_docs_core::commands::effective::{self, Options, Reading};
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
        dependents,
    };
    let reading = effective::read(store.as_ref(), docs_dir, &options, mode.is_json());
    write_stdout_bytes(reading.output.as_bytes());
    if let Some(topic) = options.topic.as_deref().filter(|_| !quiet) {
        report_no_match(&reading, topic, docs_dir);
    }
    ExitCode::SUCCESS
}

/// Tells the caller on stderr that a topic matched no live record, so an
/// empty stdout is not mistaken for an empty bundle.
fn report_no_match(reading: &Reading, topic: &str, docs_dir: &Path) {
    if reading.matched > 0 {
        return;
    }
    eprintln!(
        "living-docs read records: no live record mentions '{topic}' ({} retired withheld); try a broader term or `grep -ril '{topic}' {}`",
        reading.withheld,
        docs_dir.display()
    );
}
