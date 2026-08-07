//! `check` verb wrapper: resolves the bundle path, then delegates to `living_docs_core::check::run`.

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::check;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_check(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    paths: Vec<PathBuf>,
) -> ExitCode {
    let bundle = check_bundle(backend, docs_dir, paths);
    match build_backend_store(backend, engine, &bundle) {
        Ok(store) => check::run(store.as_ref(), &bundle),
        Err(err) => report_failure(&err),
    }
}

/// The db backend has no notion of `check`'s `[BUNDLE_ROOT]` positional
/// argument — its `DocStore` is scoped to `--docs-dir` at construction — so
/// it always checks `docs_dir`, ignoring `paths`. The fs backend prefers a
/// positional `[BUNDLE_ROOT]` when given one, and otherwise falls back to
/// `docs_dir` — so `--docs-dir X fmt`/`check` operates on `X`, never a
/// hardcoded `docs`.
pub(crate) fn check_bundle(backend: Backend, docs_dir: &Path, paths: Vec<PathBuf>) -> PathBuf {
    match backend {
        Backend::Db => docs_dir.to_path_buf(),
        Backend::Fs => paths
            .into_iter()
            .next()
            .unwrap_or_else(|| docs_dir.to_path_buf()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bundle_uses_docs_dir_for_the_db_backend_ignoring_paths() {
        let bundle = check_bundle(
            Backend::Db,
            Path::new("/repo/docs"),
            vec![PathBuf::from("/ignored")],
        );
        assert_eq!(bundle, PathBuf::from("/repo/docs"));
    }

    #[test]
    fn check_bundle_uses_the_first_path_argument_for_the_fs_backend() {
        let bundle = check_bundle(
            Backend::Fs,
            Path::new("/repo/docs"),
            vec![PathBuf::from("/bundle")],
        );
        assert_eq!(bundle, PathBuf::from("/bundle"));
    }

    #[test]
    fn check_bundle_falls_back_to_docs_dir_for_the_fs_backend_when_no_paths_are_given() {
        let bundle = check_bundle(Backend::Fs, Path::new("/repo/docs"), Vec::new());
        assert_eq!(bundle, PathBuf::from("/repo/docs"));
    }

    #[test]
    fn check_bundle_honors_a_custom_docs_dir_for_the_fs_backend_when_no_paths_are_given() {
        let bundle = check_bundle(Backend::Fs, Path::new("/repo/custom"), Vec::new());
        assert_eq!(bundle, PathBuf::from("/repo/custom"));
    }
}
