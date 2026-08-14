//! `migrate` verb wrapper: resolves the bundle path exactly like `check`,
//! then delegates to the read-only advisor `living_docs_core::commands::migrate::run`.
//! With `--apply` (ADR 0040) it becomes an fs-only transaction over the
//! mechanical subset: snapshot every `.md` plus the seal ledger, run
//! `index` then `fmt`, and roll back byte-for-byte on any failure or a
//! `check` regression. The core advisor stays read-only; `AUTHOR` steps are
//! never applied, and an `ADOPT` plan refuses `--apply`.

use crate::commands::check::check_bundle;
use crate::config::{Backend, Engine};
use crate::store::{build_backend_store, report_failure};
use living_docs_core::{check, commands, seal};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{fs, io};

pub(crate) fn run_migrate(
    backend: Backend,
    engine: Engine,
    docs_dir: &Path,
    paths: Vec<PathBuf>,
    apply: bool,
) -> ExitCode {
    let bundle = check_bundle(backend, docs_dir, paths);
    if apply {
        return run_apply(backend, engine, &bundle);
    }
    match build_backend_store(backend, engine, &bundle) {
        Ok(store) => commands::migrate::run(store.as_ref(), &bundle),
        Err(err) => report_failure(&err),
    }
}

fn run_apply(backend: Backend, engine: Engine, bundle: &Path) -> ExitCode {
    if matches!(backend, Backend::Db) {
        eprintln!(
            "living-docs migrate --apply: fs backend only — db-mode writes are already transactional per record and have no bundle-wide snapshot (ADR 0040)"
        );
        return ExitCode::from(2);
    }
    let store = match build_backend_store(backend, engine, bundle) {
        Ok(store) => store,
        Err(err) => return report_failure(&err),
    };
    let steps = commands::migrate::plan(store.as_ref(), bundle);
    apply_plan(store.as_ref(), bundle, &steps)
}

fn apply_plan(
    store: &dyn living_docs_core::store::DocStore,
    bundle: &Path,
    steps: &[String],
) -> ExitCode {
    if steps.iter().any(|step| step.starts_with("ADOPT ")) {
        eprintln!(
            "living-docs migrate --apply: no bundle to adapt — adoption is judgment plus user confirmation; follow the ADOPT steps from `living-docs migrate` (ADR 0040)"
        );
        return ExitCode::from(2);
    }
    if steps.is_empty() {
        println!("Bundle is current — nothing to apply.");
        return ExitCode::SUCCESS;
    }
    let before = check::check_violations(store, bundle).len();
    let snapshot = match Snapshot::take(bundle) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            eprintln!("living-docs migrate --apply: snapshot failed: {err}");
            return ExitCode::from(2);
        }
    };
    match run_mechanical(store, bundle, before) {
        Ok(after) => report_applied(steps, before, after),
        Err(message) => rollback(&snapshot, bundle, &message),
    }
}

/// Runs the mechanical subset and returns the after-apply violation count;
/// `Err` carries the reason a rollback is required — a failed verb or a
/// `check` regression past the before-apply count.
fn run_mechanical(
    store: &dyn living_docs_core::store::DocStore,
    bundle: &Path,
    before: usize,
) -> Result<usize, String> {
    if !succeeded(commands::index::run(store, bundle, None, None)) {
        return Err("`index` failed".to_string());
    }
    if !succeeded(commands::fmt::run(store, bundle)) {
        return Err("`fmt` failed".to_string());
    }
    let after = check::check_violations(store, bundle).len();
    if after > before {
        return Err(format!(
            "`check` regressed from {before} to {after} violation(s)"
        ));
    }
    Ok(after)
}

fn report_applied(steps: &[String], before: usize, after: usize) -> ExitCode {
    println!("APPLIED living-docs index");
    println!("APPLIED living-docs fmt");
    println!("APPLIED living-docs check — {before} violation(s) before, {after} after");
    let author: Vec<&String> = steps
        .iter()
        .filter(|step| step.starts_with("AUTHOR "))
        .collect();
    if !author.is_empty() {
        println!();
        println!("Remaining judgment steps (never applied automatically):");
        for step in author {
            println!("  {step}");
        }
    }
    ExitCode::SUCCESS
}

fn rollback(snapshot: &Snapshot, bundle: &Path, reason: &str) -> ExitCode {
    match snapshot.restore(bundle) {
        Ok(()) => eprintln!(
            "living-docs migrate --apply: ROLLED BACK — {reason}; bundle and seal ledger restored byte-for-byte"
        ),
        Err(err) => eprintln!(
            "living-docs migrate --apply: {reason}; AND the rollback itself failed: {err} — restore from git"
        ),
    }
    ExitCode::from(1)
}

/// `std::process::ExitCode` exposes no comparison; the debug form is the
/// same stable proxy the core's own tests use.
fn succeeded(code: ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

/// A byte-for-byte snapshot of every `.md` under the bundle plus the seal
/// ledger (ADR 0039) — restoring deletes files created after the snapshot,
/// rewrites changed ones, and puts the ledger back exactly as it was.
struct Snapshot {
    files: BTreeMap<PathBuf, Vec<u8>>,
    ledger: Option<(PathBuf, Option<Vec<u8>>)>,
}

impl Snapshot {
    fn take(bundle: &Path) -> io::Result<Self> {
        let mut files = BTreeMap::new();
        collect_md(bundle, &mut files)?;
        let ledger = seal::seal_dir_for(bundle).map(|dir| {
            let path = dir.join("seals.json");
            let bytes = fs::read(&path).ok();
            (path, bytes)
        });
        Ok(Self { files, ledger })
    }

    fn restore(&self, bundle: &Path) -> io::Result<()> {
        let mut current = BTreeMap::new();
        collect_md(bundle, &mut current)?;
        for path in current.keys() {
            if !self.files.contains_key(path) {
                fs::remove_file(path)?;
            }
        }
        for (path, bytes) in &self.files {
            fs::write(path, bytes)?;
        }
        match &self.ledger {
            Some((path, Some(bytes))) => fs::write(path, bytes)?,
            Some((path, None)) if path.exists() => fs::remove_file(path)?,
            Some(_) | None => {}
        }
        Ok(())
    }
}

fn collect_md(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_md(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.insert(path.clone(), fs::read(&path)?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
