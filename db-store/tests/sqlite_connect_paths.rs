//! Public-API regression coverage for [`db_store::connect`]'s SQLite path
//! resolution (ADR 0004, issue 0030): a bare relative default URL resolves
//! against the working directory into the intended `.living-docs` parent,
//! and a caller-side double-scheme wrap (`sqlite://sqlite://…`) must never
//! let `connect` create a literal `sqlite:`-named directory. Split out of
//! `db-store/src/lib.rs`'s inline test module to keep that file within the
//! file-size ratchet (issue 0028).

use std::path::PathBuf;
use std::sync::Mutex;

use db_store::connect;
use sea_orm::{ConnectionTrait, DbBackend};

fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let path = std::env::temp_dir()
        .join(format!("living-docs-db-store-connect-test-{label}-{nanos}"))
        .join("index.db");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    (path, url)
}

#[tokio::test]
async fn connect_creates_missing_parent_dirs_for_a_sqlite_file_url() {
    let (db_path, db_url) = temp_sqlite_url("parent-dir-creation");
    assert!(!db_path.parent().expect("path has a parent").exists());

    let conn = connect(&db_url).await.expect("connect creates parent dirs");
    assert_eq!(conn.get_database_backend(), DbBackend::Sqlite);

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
}

#[tokio::test]
async fn connect_infers_sqlite_backend_from_a_file_url() {
    let (db_path, db_url) = temp_sqlite_url("backend-inference");

    let conn = connect(&db_url).await.expect("connect");
    assert_eq!(conn.get_database_backend(), DbBackend::Sqlite);

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
}

/// Serializes the two tests below that mutate the process-wide current
/// working directory — a test binary runs its tests on multiple threads in
/// one process, and `std::env::set_current_dir` has no per-thread scope.
static CWD_GUARD: Mutex<()> = Mutex::new(());

fn temp_scratch_cwd(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-db-store-cwd-{label}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create scratch working directory");
    dir
}

/// Runs an async future to completion on a dedicated current-thread runtime
/// — kept synchronous (`#[test]`, not `#[tokio::test]`) so the [`CWD_GUARD`]
/// lock never spans an `.await` point.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build test runtime")
        .block_on(future)
}

#[test]
fn connect_default_url_from_a_scratch_cwd_creates_the_living_docs_parent_and_no_sqlite_entry() {
    let _guard = CWD_GUARD.lock().expect("cwd guard mutex is not poisoned");
    let scratch = temp_scratch_cwd("default-url-guard");
    let original_cwd = std::env::current_dir().expect("read the current working directory");
    std::env::set_current_dir(&scratch).expect("switch to the scratch working directory");

    let result = block_on(connect("sqlite://.living-docs/index.db?mode=rwc"));

    std::env::set_current_dir(&original_cwd).expect("restore the original working directory");
    result.expect("connect succeeds against the default sqlite url");

    assert!(
        !scratch.join("sqlite:").exists(),
        "connect must never create a literal `sqlite:` directory"
    );
    assert!(
        scratch.join(".living-docs").is_dir(),
        "connect must still create the intended `.living-docs` parent directory"
    );

    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn connect_double_scheme_url_from_a_scratch_cwd_creates_no_sqlite_entry() {
    let _guard = CWD_GUARD.lock().expect("cwd guard mutex is not poisoned");
    let scratch = temp_scratch_cwd("double-scheme-guard");
    let original_cwd = std::env::current_dir().expect("read the current working directory");
    std::env::set_current_dir(&scratch).expect("switch to the scratch working directory");

    let _ = block_on(connect(
        "sqlite://sqlite://.living-docs/index.db?mode=rwc?mode=rwc",
    ));

    std::env::set_current_dir(&original_cwd).expect("restore the original working directory");

    assert!(
        !scratch.join("sqlite:").exists(),
        "connect must never create a literal `sqlite:` directory for a double-scheme url"
    );

    let _ = std::fs::remove_dir_all(&scratch);
}
