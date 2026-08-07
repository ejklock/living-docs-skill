//! `next` verb wrapper: resolves the doc-type token to its directory, then
//! delegates to `living_docs_core::commands::next::run`.

use crate::store::report_failure;
use living_docs_core::commands;
use living_docs_core::paths;
use std::path::Path;
use std::process::ExitCode;

/// `commands::next::run` scans `docs_dir/doc_type` for the highest existing
/// `NNNN` prefix, where `doc_type` must already be the record DIRECTORY name
/// (e.g. `issues`), not the CLI token (e.g. `issue`) — [`paths::dir_for`]
/// resolves the token the same way `new` does, so `next issue` reports one
/// past the highest existing issue number instead of always `0001`.
pub(crate) fn run_next(docs_dir: &Path, doc_type: &str) -> ExitCode {
    match paths::dir_for(doc_type) {
        Some(dir) => commands::next::run(docs_dir, dir),
        None => report_failure(&format!("unknown doc type '{doc_type}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_bundle(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("living-docs-cli-next-{label}-{nanos}"))
    }

    #[test]
    fn run_next_fails_cleanly_for_an_unknown_doc_type_token() {
        let bundle = temp_bundle("unknown-token");

        let exit_code = run_next(&bundle, "glossary");

        assert_eq!(exit_code, ExitCode::FAILURE);
    }
}
