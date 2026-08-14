//! `migrate` verb wrapper: resolves the bundle path exactly like `check`,
//! then delegates to the read-only advisor `living_docs_core::commands::migrate::run`.

use crate::commands::check::check_bundle;
use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::commands;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_migrate(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    paths: Vec<PathBuf>,
) -> ExitCode {
    let bundle = check_bundle(backend, docs_dir, paths);
    match build_backend_store(backend, engine, &bundle) {
        Ok(store) => commands::migrate::run(store.as_ref(), &bundle),
        Err(err) => report_failure(&err),
    }
}
