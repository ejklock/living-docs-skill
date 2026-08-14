//! `new` verb wrapper: fs-backend delegates to core, db-backend plans then commits (ADR 0016).

use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, build_db_doc_store, report_failure};
use living_docs_core::commands;
use living_docs_core::commands::new::NewOptions;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

/// Resolves the `--json` argument's payload forms (ADR 0038): `-` reads
/// stdin, `@<path>` reads a file, anything else is the literal JSON.
fn resolve_json_payload(json: Option<&str>) -> Result<Option<String>, String> {
    match json {
        None => Ok(None),
        Some("-") => {
            let mut payload = String::new();
            std::io::stdin()
                .read_to_string(&mut payload)
                .map_err(|e| format!("--json -: reading stdin failed: {e}"))?;
            Ok(Some(payload))
        }
        Some(arg) => match arg.strip_prefix('@') {
            Some(path) => std::fs::read_to_string(path)
                .map(Some)
                .map_err(|e| format!("--json @{path}: {e}")),
            None => Ok(Some(arg.to_string())),
        },
    }
}

/// The `new` verb's optional CLI arguments, bundled so the wrapper's
/// signature stays within the argument budget as flags accrue.
pub(crate) struct NewArgs<'a> {
    pub(crate) description: Option<&'a str>,
    pub(crate) kind: Option<&'a str>,
    pub(crate) json: Option<&'a str>,
}

pub(crate) fn run_new(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    args: &NewArgs,
) -> ExitCode {
    let payload = match resolve_json_payload(args.json) {
        Ok(payload) => payload,
        Err(err) => return report_new_db_failure(&err),
    };
    let opts = NewOptions {
        description: args.description,
        kind: args.kind,
        sections_json: payload.as_deref(),
    };
    match backend {
        Backend::Fs => match build_backend_store(backend, engine, docs_dir) {
            Ok(store) => commands::new::run(store.as_ref(), docs_dir, doc_type, title, &opts),
            Err(err) => report_failure(&err),
        },
        Backend::Db => run_new_db(engine, docs_dir, doc_type, title, &opts),
    }
}

/// `--backend db new`'s own path: unlike `Backend::Fs` (which delegates
/// straight to [`commands::new::run`]'s plain [`living_docs_core::store::DocStore::write`]),
/// db-mode plans the target path with [`commands::new::plan`] and commits it
/// through [`db_store::DbDocStore::write_checked`], so an invalid record is
/// rejected before it is ever visible (ADR 0016, issue 0010 slice 2).
fn run_new_db(
    engine: Engine,
    docs_dir: &Path,
    doc_type: &str,
    title: &str,
    opts: &NewOptions,
) -> ExitCode {
    let store = match build_db_doc_store(engine, docs_dir) {
        Ok(store) => store,
        Err(err) => return report_failure(&err),
    };
    match commands::new::plan(&store, docs_dir, doc_type, title, opts) {
        Ok((target_path, filled)) => commit_new_db(&store, &target_path, &filled),
        Err(err) => report_new_db_failure(&err),
    }
}

fn commit_new_db(store: &db_store::DbDocStore, target_path: &Path, filled: &str) -> ExitCode {
    match store.write_checked(target_path, filled) {
        Ok(_) => {
            println!("{}", target_path.display());
            println!("{}", commands::new::BODY_ONLY_INSTRUCTION);
            ExitCode::SUCCESS
        }
        Err(err) => report_new_db_failure(&err.to_string()),
    }
}

/// Mirrors [`commands::new::run`]'s own failure wording exactly, so
/// db-mode's `plan`/`write_checked` errors print and exit identically to
/// fs-mode's `scaffold` errors — the only new outcome db-mode can now reach
/// that fs-mode never could is a failing `check` from `write_checked`.
fn report_new_db_failure(message: &str) -> ExitCode {
    eprintln!("living-docs new: {message}");
    ExitCode::from(2)
}
