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
use std::path::{Path, PathBuf};

use living_docs_core::store::SearchIndex;
use sea_orm::{Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

use migration::Migrator;

pub use record::{ExtractedRecord, SearchHit};
pub use search::search;
pub use sync::sync;

/// Result alias for this crate's fallible operations, using SeaORM's own
/// error type since every failure here originates from the database layer.
pub type Result<T> = std::result::Result<T, DbErr>;

/// Opens a SQLite connection at `db_path`, creating the database file and any
/// missing parent directories on first use.
pub async fn connect(db_path: &Path) -> Result<DatabaseConnection> {
    if let Some(parent) = db_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|err| DbErr::Custom(err.to_string()))?;
    }
    Database::connect(format!("sqlite://{}?mode=rwc", db_path.display())).await
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
    /// Opens `db_path` on a dedicated current-thread runtime and wraps the
    /// resulting connection for synchronous search.
    pub fn new(db_path: &Path) -> io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let conn = runtime
            .block_on(connect(db_path))
            .map_err(io::Error::other)?;
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

    fn temp_db_path(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("living-docs-db-store-test-{label}-{nanos}.db"))
    }

    #[test]
    fn db_search_index_bridges_the_sync_search_index_port_without_an_ambient_runtime() {
        let db_path = temp_db_path("search-index-bridge");

        let setup_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build setup runtime");
        setup_runtime.block_on(async {
            let conn = connect(&db_path).await.expect("connect");
            migrate(&conn).await.expect("migrate");
            let (store, bundle) = sync::test_support::seeded_corpus();
            sync::sync(&conn, &store, &bundle).await.expect("sync");
        });
        drop(setup_runtime);

        let index = DbSearchIndex::new(&db_path).expect("build search index");
        let hits = index.search("quokka").expect("search should succeed");

        assert_eq!(hits, vec![PathBuf::from("adr/0001-quokka-caching.md")]);

        let no_hits = index
            .search("zzzznomatch")
            .expect("no-match search should succeed");
        assert!(no_hits.is_empty());

        let _ = std::fs::remove_file(&db_path);
    }
}
