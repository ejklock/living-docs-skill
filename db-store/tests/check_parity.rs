//! `check` now reads every record's content through `DocStore::read`
//! (issue 0006 slice 0006-D1, closing the slice 0006-B/B2 gap where
//! `check::run` still read `std::fs` directly regardless of the backend it
//! was given). This file proves both directions: file-mode `check` over an
//! `FsStore`-equivalent is unchanged (no regression, AC D1a), and db-mode
//! `check` over a [`DbDocStore`] validates the serialized DB projection
//! rather than the on-disk tree — a supersede target absent only from the
//! DB is reported even while the on-disk tree stays clean (AC D1b).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use db_store::{connect, migrate, sync_project, DbDocStore};
use living_docs_core::check;
use living_docs_core::store::DocStore;

/// Mirrors `fs-store`'s recursive `.md` walk (not reused directly — every
/// test file in this crate carries its own small local `DocStore`, the
/// established convention in `ingestion.rs`/`dual_engine.rs`/`parity.rs`).
struct LocalFsStore;

impl DocStore for LocalFsStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        collect_md_files(root, &mut found);
        found.sort();
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }
}

/// A `LocalFsStore` whose `list` omits `excluded`, so `sync_project` never
/// inserts a record for it — used to simulate a record that is present on
/// disk but absent from the active backend's own projection.
struct PartialFsStore {
    excluded: PathBuf,
}

impl DocStore for PartialFsStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut found = LocalFsStore.list(root)?;
        found.retain(|path| path != &self.excluded);
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        LocalFsStore.read(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        LocalFsStore.write(path, contents)
    }
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn exit_code_is_success(code: &ExitCode) -> bool {
    format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
}

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-check-parity-{label}-{nanos}"))
        .join("index.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    (path, url)
}

fn cleanup_sqlite_file(db_path: &Path) {
    let _ = fs::remove_file(db_path);
    if let Some(parent) = db_path.parent() {
        let _ = fs::remove_dir(parent);
    }
}

fn scratch_bundle_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("living-docs-check-parity-bundle-{label}-{nanos}"))
}

fn write_scratch_doc(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create scratch bundle directory");
    }
    fs::write(path, contents).expect("write scratch bundle file");
}

/// Runs `migrate` + `sync_project` on a dedicated setup runtime, then drops
/// it — `DbDocStore` opens its own runtime + connection afterward, and a
/// sync trait method's `block_on` panics if called from inside an
/// already-running tokio runtime (mirrors `parity.rs`'s `setup_synced_db`).
/// A file-backed SQLite database, not `sqlite::memory:`, is required so the
/// two connections see the same data.
fn setup_synced_db(url: &str, store: &dyn DocStore, bundle: &Path) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build setup runtime");
    runtime.block_on(async {
        let conn = connect(url).await.expect("connect");
        migrate(&conn).await.expect("migrate");
        sync_project(&conn, store, bundle, "default")
            .await
            .expect("sync project");
    });
}

const OLD_DOC: &str = "---\ntype: ADR\ntitle: Quokka Caching Strategy\ndescription: d.\nstatus: Superseded\nsuperseded_by: 0002\n---\n# 0001. Quokka Caching Strategy\n\nBody.\n";
const NEW_DOC: &str = "---\ntype: ADR\ntitle: Improved Caching Strategy\ndescription: d.\nstatus: Accepted\nsupersedes: 0001\n---\n# 0002. Improved Caching Strategy\n\nBody.\n";

/// A clean, complete two-record bundle: both records exist, the supersede
/// chain resolves, directory membership and reachability are satisfied.
fn clean_supersede_bundle(root: &Path) {
    write_scratch_doc(
        &root.join("index.md"),
        "# Index\n\n- [ADRs](/adr/index.md)\n",
    );
    write_scratch_doc(
        &root.join("adr").join("index.md"),
        "# ADR Index\n\n- [Quokka](/adr/0001-quokka-caching.md)\n- [Improved](/adr/0002-improved-caching.md)\n",
    );
    write_scratch_doc(&root.join("adr").join("0001-quokka-caching.md"), OLD_DOC);
    write_scratch_doc(&root.join("adr").join("0002-improved-caching.md"), NEW_DOC);
}

/// A minimal bundle with one seeded violation: `adr/0001-broken.md` carries
/// no frontmatter at all, which `check`'s per-file OKF invariant rejects.
fn seed_frontmatter_violation_bundle(root: &Path) {
    write_scratch_doc(
        &root.join("index.md"),
        "# Index\n\n- [ADRs](/adr/index.md)\n",
    );
    write_scratch_doc(
        &root.join("adr").join("index.md"),
        "# ADR Index\n\n- [Broken](/adr/0001-broken.md)\n",
    );
    write_scratch_doc(
        &root.join("adr").join("0001-broken.md"),
        "# Broken\n\nThis record carries no frontmatter at all.\n",
    );
}

#[test]
fn check_over_a_fully_synced_db_store_matches_check_over_fs_store_on_a_clean_corpus() {
    let root = scratch_bundle_root("clean-full-sync");
    clean_supersede_bundle(&root);

    let (db_path, db_url) = temp_sqlite_url("clean-full-sync");
    setup_synced_db(&db_url, &LocalFsStore, &root);
    let db_store = DbDocStore::new(&db_url, root.clone()).expect("open db doc store");

    let fs_verdict = check::run(&LocalFsStore, &root);
    let db_verdict = check::run(&db_store, &root);

    assert!(
        exit_code_is_success(&fs_verdict),
        "sanity: the on-disk corpus must be clean"
    );
    assert_eq!(
        format!("{fs_verdict:?}"),
        format!("{db_verdict:?}"),
        "check must reach the same verdict over fs-store and a fully synced db-store"
    );

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn check_over_a_fully_synced_db_store_fails_identically_to_fs_store_on_a_seeded_frontmatter_violation(
) {
    let root = scratch_bundle_root("violation-full-sync");
    seed_frontmatter_violation_bundle(&root);

    let (db_path, db_url) = temp_sqlite_url("violation-full-sync");
    setup_synced_db(&db_url, &LocalFsStore, &root);
    let db_store = DbDocStore::new(&db_url, root.clone()).expect("open db doc store");

    let fs_verdict = check::run(&LocalFsStore, &root);
    let db_verdict = check::run(&db_store, &root);

    assert!(
        !exit_code_is_success(&fs_verdict),
        "sanity: fs-store must fail on the seeded violation"
    );
    assert_eq!(
        format!("{fs_verdict:?}"),
        format!("{db_verdict:?}"),
        "a seeded frontmatter violation must fail identically on both backends"
    );

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn check_over_db_store_fails_on_a_supersede_target_absent_only_from_the_db_projection() {
    let root = scratch_bundle_root("db-only-violation");
    clean_supersede_bundle(&root);
    let target = root.join("adr").join("0002-improved-caching.md");

    let (db_path, db_url) = temp_sqlite_url("db-only-violation");
    let partial_store = PartialFsStore {
        excluded: target.clone(),
    };
    setup_synced_db(&db_url, &partial_store, &root);
    let db_store = DbDocStore::new(&db_url, root.clone()).expect("open db doc store");

    let fs_verdict = check::run(&LocalFsStore, &root);
    let db_verdict = check::run(&db_store, &root);

    assert!(
        exit_code_is_success(&fs_verdict),
        "the on-disk tree stays clean — the supersede target file is still there, just unsynced"
    );
    assert!(
        !exit_code_is_success(&db_verdict),
        "check over db-store must catch the supersede target missing from the DB projection, \
         even though the same-named file still exists on disk"
    );

    cleanup_sqlite_file(&db_path);
    let _ = fs::remove_dir_all(&root);
}
