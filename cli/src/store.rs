//! Store construction, db-project bootstrap, the Tokio runtime, and the shared failure reporter.

use crate::config::{Backend, Engine};
use living_docs_core::store::DocStore;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn build_backend_store(
    backend: Backend,
    engine: Engine,
    root: &Path,
) -> Result<Box<dyn DocStore>, String> {
    match backend {
        Backend::Fs => Ok(Box::new(SealingStore {
            inner: fs_store::FsStore::new(),
        })),
        Backend::Db => {
            build_db_doc_store(engine, root).map(|store| Box::new(store) as Box<dyn DocStore>)
        }
    }
}

/// Opens (migrating if needed) the db backend's connection, bootstraps
/// `root`'s project if this is its first use, then hands back a
/// [`db_store::DbDocStore`] scoped to it — the `--backend db` counterpart
/// of [`fs_store::FsStore::new`]. `engine` resolves the connection string
/// exactly as `db sync`/`search` do (ADR 0004: ParadeDB default, SQLite
/// opt-in), so `--backend db` authoring honors the same `--engine` choice.
pub(crate) fn build_db_doc_store(
    engine: Engine,
    root: &Path,
) -> Result<db_store::DbDocStore, String> {
    let url = engine.resolve_url()?;
    let project_slug = crate::commands::db::derive_project_slug(root);
    let runtime = build_runtime().map_err(|e| e.to_string())?;
    runtime
        .block_on(prepare_db_project(&url, root, &project_slug))
        .map_err(|e| e.to_string())?;
    db_store::DbDocStore::for_project(&url, root.to_path_buf(), &project_slug)
        .map_err(|e| e.to_string())
}

/// Ensures `project_slug` exists before a [`db_store::DbDocStore`] is
/// constructed over it — its constructor only looks an existing project up,
/// it never creates one. Bootstraps via an [`EmptyStore`] rather than
/// [`fs_store::FsStore`] so a first `--backend db` call never silently
/// ingests whatever `.md` files happen to sit under `root`; only ever
/// creates the project shell (never clears an existing one, since it is
/// skipped entirely once found).
async fn prepare_db_project(url: &str, root: &Path, project_slug: &str) -> db_store::Result<()> {
    let conn = db_store::connect(url).await?;
    db_store::migrate(&conn).await?;
    let existing = db_store::list_projects(&conn).await?;
    if existing.iter().any(|project| project.slug == project_slug) {
        return Ok(());
    }
    db_store::sync_project(&conn, &EmptyStore, root, project_slug)
        .await
        .map(|_| ())
}

/// A [`DocStore`] with no records, used only to bootstrap a fresh project
/// row for `--backend db` (via [`db_store::sync_project`]) without
/// ingesting anything from disk.
struct EmptyStore;

impl DocStore for EmptyStore {
    fn list(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn read(&self, _path: &Path) -> io::Result<String> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "empty store carries no records",
        ))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn build_runtime() -> std::io::Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
}

pub(crate) fn report_failure(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::FAILURE
}

/// Decorates the fs store so every CLI record write re-seals its provenance
/// (ADR 0039) — the single choke point that keeps `new`/`brief`/`status`/
/// `supersede`/`fmt`/`describe` sealed with zero per-verb wiring. A no-op
/// outside a git repository or before `living-docs seal init`.
struct SealingStore {
    inner: fs_store::FsStore,
}

impl DocStore for SealingStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.inner.list(root)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.inner.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.inner.write(path, contents)?;
        if let Some(seal_dir) = living_docs_core::seal::seal_dir_for(path) {
            living_docs_core::seal::seal_record(&seal_dir, path, contents);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_store_lists_no_records_and_refuses_every_read() {
        let store = EmptyStore;
        assert!(store
            .list(Path::new("/bundle"))
            .expect("empty store lists successfully")
            .is_empty());
        assert!(store.read(Path::new("/bundle/adr/0001-x.md")).is_err());
        assert!(store.write(Path::new("/bundle/adr/0001-x.md"), "x").is_ok());
    }
}
