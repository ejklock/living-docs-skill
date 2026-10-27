//! `read` verb wrapper: delegates to `living_docs_core::commands::effective`
//! to compile and print the agent-facing view, as colored text or JSON per the
//! resolved output mode, and explains an empty `--topic` result on stderr.

use crate::args::ReadArgs;
use crate::output::{write_stdout_bytes, OutputMode};
use crate::store::build_store;
use living_docs_core::commands::effective::{self, Dependents, Options, Reading};
use living_docs_core::store::DocStore;
use std::path::Path;
use std::process::ExitCode;

pub(crate) fn run_read(docs_dir: &Path, args: ReadArgs, mode: OutputMode, quiet: bool) -> ExitCode {
    let store = build_store(docs_dir);
    let dependents = match dependents_of(store.as_ref(), docs_dir, &args) {
        Ok(dependents) => dependents,
        Err(message) => {
            eprintln!("living-docs read: {message}");
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

/// The dependents query the flags name, with its reference resolved to a
/// record path; a reference naming no record, or several, is the error.
fn dependents_of(
    store: &dyn DocStore,
    docs_dir: &Path,
    args: &ReadArgs,
) -> Result<Option<Dependents>, String> {
    let resolve = |reference: &str| effective::resolve_target(store, docs_dir, reference);
    if let Some(reference) = &args.links_to {
        return resolve(reference).map(|path| Some(Dependents::LinksTo(path)));
    }
    match &args.impact {
        Some(reference) => resolve(reference).map(|path| Some(Dependents::Impact(path))),
        None => Ok(None),
    }
}

/// Tells the caller on stderr that a topic matched no live record, so an
/// empty stdout is not mistaken for an empty bundle.
fn report_no_match(reading: &Reading, topic: &str, docs_dir: &Path) {
    if reading.matched > 0 {
        return;
    }
    eprintln!(
        "living-docs read: no live record mentions '{topic}' ({} retired withheld); try a broader term or `grep -ril '{topic}' {}`",
        reading.withheld,
        docs_dir.display()
    );
}
