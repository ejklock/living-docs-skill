//! `fmt` verb wrapper: fs-backend only, reuses `check::check_bundle` for bundle resolution.

use crate::commands::check::check_bundle;
use crate::config::Backend;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// `fmt` is fs-backend only (db-mode is canonical by construction on
/// export), so it needs no `build_backend_store`/`Engine` plumbing — it
/// reuses [`check_bundle`]'s `[BUNDLE_ROOT]` resolution (a positional path
/// wins; otherwise `--docs-dir`) against a fixed [`fs_store::FsStore`], the
/// same way [`crate::commands::leak_gate::run_leak_gate`] always inspects a
/// materialized filesystem bundle regardless of `--backend`.
pub(crate) fn run_fmt(docs_dir: &Path, paths: Vec<PathBuf>) -> ExitCode {
    let bundle = check_bundle(Backend::Fs, docs_dir, paths);
    living_docs_core::commands::fmt::run(&fs_store::FsStore::new(), &bundle)
}
