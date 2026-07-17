//! `db-store`: the SQLite/FTS5 derived read-model adapter (ADR 0004, issue
//! 0002). Slice S2a landed the schema foundation (connect + migrate). Slice
//! S2b adds the idempotent full-rebuild `sync`, ranked FTS5 `search`, and the
//! `SearchIndex` port implementation. CLI wiring lands in S2c.

pub mod entity;
pub mod migration;
pub mod record;
pub mod search;
pub mod sync;

use std::io;
use std::path::PathBuf;

use living_docs_core::store::SearchIndex;
use sea_orm::{ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use sea_orm_migration::MigratorTrait;

use entity::{Column, Entity as Records};
use migration::Migrator;

pub use record::{ExtractedRecord, SearchHit};
pub use search::search;
pub use sync::sync;

/// A single record's title and markdown source body, looked up by its
/// bundle-relative path (ADR 0006, issue 0003 slice S3b).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordView {
    pub title: String,
    pub body: String,
}

/// Looks up the record at `path`. Returns `None` when no record exists at
/// that path — a missing record is not an error.
pub async fn record_by_path(conn: &DatabaseConnection, path: &str) -> Result<Option<RecordView>> {
    let record = Records::find()
        .filter(Column::Path.eq(path))
        .one(conn)
        .await?;
    Ok(record.map(|model| RecordView {
        title: model.title,
        body: model.body,
    }))
}

/// Result alias for this crate's fallible operations, using SeaORM's own
/// error type since every failure here originates from the database layer.
pub type Result<T> = std::result::Result<T, DbErr>;

/// Opens a database connection at `url`, inferring the backend from its
/// scheme (`sqlite://…` or `postgres://…`, ADR 0004 issue 0004). For a
/// SQLite file URL, creates any missing parent directories before handing
/// the URL to SeaORM unchanged; other schemes are passed straight through.
pub async fn connect(url: &str) -> Result<DatabaseConnection> {
    if let Some(path) = sqlite_file_path(url) {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|err| DbErr::Custom(err.to_string()))?;
        }
    }
    Database::connect(url).await
}

/// Extracts the filesystem path from a SQLite file URL (`sqlite://<path>`,
/// optionally followed by a `?query`), or `None` for the in-memory form
/// (`sqlite::memory:`) and every non-SQLite scheme.
fn sqlite_file_path(url: &str) -> Option<PathBuf> {
    let rest = url.strip_prefix("sqlite://")?;
    let path = rest.split('?').next().unwrap_or(rest);
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Opens an in-memory SQLite connection for tests. State is discarded once
/// the returned connection is dropped.
pub async fn connect_in_memory() -> Result<DatabaseConnection> {
    Database::connect("sqlite::memory:").await
}

/// Applies every pending migration to `conn`, creating the `records` table
/// and the `records_fts` FTS5 virtual table on a fresh database.
pub async fn migrate(conn: &DatabaseConnection) -> Result<()> {
    Migrator::up(conn, None).await
}

/// Bridges the synchronous `living_docs_core::store::SearchIndex` port onto
/// this crate's async [`search`]. Holds its own dedicated current-thread
/// Tokio runtime *and* connects through it: `SearchIndex::search` is a sync
/// trait method that may be invoked from a caller with no async context at
/// all, so it cannot assume one exists — and a sea-orm connection is not
/// safe to drive from a Tokio runtime other than the one that created it, so
/// the connection and the runtime that opened it are kept together and
/// never split.
pub struct DbSearchIndex {
    conn: DatabaseConnection,
    runtime: tokio::runtime::Runtime,
}

impl DbSearchIndex {
    /// Opens `url` on a dedicated current-thread runtime and wraps the
    /// resulting connection for synchronous search.
    pub fn new(url: &str) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let conn = runtime.block_on(connect(url)).map_err(io::Error::other)?;
        Ok(Self { conn, runtime })
    }
}

impl SearchIndex for DbSearchIndex {
    fn search(&self, query: &str) -> io::Result<Vec<PathBuf>> {
        let hits = self
            .runtime
            .block_on(search::search(&self.conn, query))
            .map_err(io::Error::other)?;
        Ok(hits
            .into_iter()
            .map(|hit| PathBuf::from(hit.path))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    async fn table_names(conn: &DatabaseConnection) -> Vec<String> {
        let statement = Statement::from_string(
            conn.get_database_backend(),
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name".to_owned(),
        );
        conn.query_all(statement)
            .await
            .expect("query sqlite_master for table names")
            .into_iter()
            .map(|row| row.try_get::<String>("", "name").expect("name column"))
            .collect()
    }

    async fn row_count(conn: &DatabaseConnection, table: &str) -> i64 {
        let statement = Statement::from_string(
            conn.get_database_backend(),
            format!("SELECT COUNT(*) AS n FROM {table}"),
        );
        conn.query_one(statement)
            .await
            .expect("query row count")
            .expect("row count query returns exactly one row")
            .try_get::<i64>("", "n")
            .expect("n column")
    }

    #[tokio::test]
    async fn migrate_creates_records_and_records_fts_tables_empty() {
        let conn = connect_in_memory()
            .await
            .expect("connect to in-memory sqlite");
        migrate(&conn).await.expect("apply migration");

        let tables = table_names(&conn).await;
        assert!(
            tables.iter().any(|name| name == "records"),
            "records table missing from sqlite_master: {tables:?}"
        );
        assert!(
            tables.iter().any(|name| name == "records_fts"),
            "records_fts table missing from sqlite_master: {tables:?}"
        );

        assert_eq!(row_count(&conn, "records").await, 0);
        assert_eq!(row_count(&conn, "records_fts").await, 0);
    }

    fn temp_sqlite_url(label: &str) -> (PathBuf, String) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("living-docs-db-store-test-{label}-{nanos}"))
            .join("index.db");
        let url = format!("sqlite://{}?mode=rwc", path.display());
        (path, url)
    }

    #[tokio::test]
    async fn record_by_path_returns_some_for_a_synced_path_and_none_for_an_unknown_path() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = sync::test_support::seeded_corpus();
        sync::sync(&conn, &store, &bundle)
            .await
            .expect("sync seeded corpus");

        let found = record_by_path(&conn, "adr/0001-quokka-caching.md")
            .await
            .expect("query record_by_path")
            .expect("record exists for the seeded path");
        assert_eq!(found.title, "Quokka Caching Strategy");
        assert!(found.body.contains("quokka caching strategy"));

        let missing = record_by_path(&conn, "adr/9999-missing.md")
            .await
            .expect("query record_by_path");
        assert!(missing.is_none());
    }

    #[test]
    fn db_search_index_bridges_the_sync_search_index_port_without_an_ambient_runtime() {
        let (db_path, db_url) = temp_sqlite_url("search-index-bridge");

        let setup_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build setup runtime");
        setup_runtime.block_on(async {
            let conn = connect(&db_url).await.expect("connect");
            migrate(&conn).await.expect("migrate");
            let (store, bundle) = sync::test_support::seeded_corpus();
            sync::sync(&conn, &store, &bundle).await.expect("sync");
        });
        drop(setup_runtime);

        let index = DbSearchIndex::new(&db_url).expect("build search index");
        let hits = index.search("quokka").expect("search should succeed");

        assert_eq!(hits, vec![PathBuf::from("adr/0001-quokka-caching.md")]);

        let no_hits = index
            .search("zzzznomatch")
            .expect("no-match search should succeed");
        assert!(no_hits.is_empty());

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
    }

    #[tokio::test]
    async fn connect_creates_missing_parent_dirs_for_a_sqlite_file_url() {
        let (db_path, db_url) = temp_sqlite_url("parent-dir-creation");
        assert!(!db_path.parent().expect("path has a parent").exists());

        let conn = connect(&db_url).await.expect("connect creates parent dirs");
        assert_eq!(conn.get_database_backend(), sea_orm::DbBackend::Sqlite);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
    }

    #[tokio::test]
    async fn connect_infers_postgres_backend_from_scheme_without_a_live_server() {
        let mut options = sea_orm::ConnectOptions::new("postgres://user:pass@localhost/db");
        options.connect_lazy(true);

        let conn = Database::connect(options)
            .await
            .expect("lazy postgres connect never touches the network");

        assert_eq!(conn.get_database_backend(), sea_orm::DbBackend::Postgres);
    }

    #[tokio::test]
    async fn connect_infers_sqlite_backend_from_a_file_url() {
        let (db_path, db_url) = temp_sqlite_url("backend-inference");

        let conn = connect(&db_url).await.expect("connect");
        assert_eq!(conn.get_database_backend(), sea_orm::DbBackend::Sqlite);

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_dir(db_path.parent().expect("path has a parent"));
    }
}
