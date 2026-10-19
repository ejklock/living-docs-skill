//! The output shared by the verbs that refresh an index after writing a record.

use crate::output::OutputMode;
use living_docs_core::commands::index::Indexed;
use std::process::ExitCode;

/// Reports the index refresh after a record write. Each written index is
/// named on stderr in text mode unless `quiet`; an index that could not be
/// written leaves the record in place, names the repair verb and exits 1, so
/// a script sees the half-finished state a usage error (exit 2) never has.
pub(crate) fn report_indexed(
    verb: &str,
    indexed: &Indexed,
    mode: OutputMode,
    quiet: bool,
) -> ExitCode {
    if !mode.is_json() && !quiet {
        for path in &indexed.written {
            eprintln!("living-docs {verb}: indexed {}", path.display());
        }
    }
    for failure in &indexed.failures {
        eprintln!("living-docs {verb}: {failure}");
    }
    match indexed.failures.is_empty() {
        true => ExitCode::SUCCESS,
        false => ExitCode::from(1),
    }
}
