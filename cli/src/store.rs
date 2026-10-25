//! Store construction over the `.md` tree and the shared failure reporter.

use living_docs_core::store::DocStore;
use std::path::Path;
use std::process::ExitCode;

/// The store for one verb run, rooted at the bundle the verb works on.
pub(crate) fn build_store(bundle: &Path) -> Box<dyn DocStore> {
    Box::new(fs_store::FsStore::rooted(bundle))
}

/// Prints a usage-level failure (an invalid or unresolvable argument, e.g.
/// an unknown `guide` skill/topic) and returns exit code 2, per the
/// documented exit-code table (ADR 0060: 0 success, 1 gate failed, 2 usage).
pub(crate) fn report_failure(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}
