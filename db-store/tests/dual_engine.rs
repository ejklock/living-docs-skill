//! Dual-engine CRUD + search fitness test (ADR 0004, issue 0004 slice
//! 0004-C): the same corpus, seeded and queried through the identical
//! `DocStore`/`sync`/`search` calls, must round-trip and rank the same top
//! hit on SQLite (always, no server required — the embedded fitness) and on
//! Postgres/ParadeDB when a live instance is configured via
//! `LIVING_DOCS_TEST_PG_URL`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use db_store::{connect, connect_in_memory, migrate, record_by_path, search, sync};
use living_docs_core::store::DocStore;
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};

struct MemoryStore {
    files: BTreeMap<PathBuf, String>,
}

impl DocStore for MemoryStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }
}

const QUOKKA_DOC: &str = "---\ntype: ADR\ntitle: Quokka Caching Strategy\ndescription: Adopt quokka-based caching for the read model.\nstatus: Accepted\n---\n# 0001. Quokka Caching Strategy\n\nWe adopt an aggressive quokka caching strategy for search results.\n";
const UNRELATED_DOC: &str = "---\ntype: ADR\ntitle: Unrelated Decision\ndescription: Something else entirely.\nstatus: Accepted\n---\n# 0002. Unrelated Decision\n\nThis document discusses logging conventions.\n";

fn seeded_corpus() -> (MemoryStore, PathBuf) {
    let bundle = PathBuf::from("/bundle");
    let mut files = BTreeMap::new();
    files.insert(
        bundle.join("adr").join("0001-quokka-caching.md"),
        QUOKKA_DOC.to_owned(),
    );
    files.insert(
        bundle.join("adr").join("0002-unrelated.md"),
        UNRELATED_DOC.to_owned(),
    );
    (MemoryStore { files }, bundle)
}

async fn assert_crud_and_search_round_trip(conn: &DatabaseConnection) {
    let (store, bundle) = seeded_corpus();
    let inserted = sync(conn, &store, &bundle)
        .await
        .expect("sync seeds the corpus through the DocStore path");
    assert_eq!(inserted, 2);

    let found = record_by_path(conn, "adr/0001-quokka-caching.md")
        .await
        .expect("query record_by_path")
        .expect("the seeded record round-trips by its bundle-relative path");
    assert_eq!(found.title, "Quokka Caching Strategy");

    let hits = search(conn, "quokka").await.expect("search");
    assert_eq!(
        hits.first().map(|hit| hit.path.as_str()),
        Some("adr/0001-quokka-caching.md"),
        "expected the quokka record to rank first, got: {hits:?}"
    );
}

#[tokio::test]
async fn sqlite_in_memory_crud_and_search_round_trip_with_no_server() {
    let conn = connect_in_memory()
        .await
        .expect("connect to in-memory sqlite");
    migrate(&conn).await.expect("migrate");

    assert_crud_and_search_round_trip(&conn).await;
}

#[tokio::test]
async fn postgres_crud_and_search_round_trip_against_a_live_paradedb() {
    let Ok(url) = std::env::var("LIVING_DOCS_TEST_PG_URL") else {
        eprintln!(
            "skipping postgres_crud_and_search_round_trip_against_a_live_paradedb: \
             LIVING_DOCS_TEST_PG_URL is not set"
        );
        return;
    };

    let conn = connect(&url).await.expect("connect to postgres");
    migrate(&conn).await.expect("migrate (idempotent)");
    conn.execute(Statement::from_string(
        conn.get_database_backend(),
        "DELETE FROM records".to_owned(),
    ))
    .await
    .expect("clear records left over from a prior run");

    assert_crud_and_search_round_trip(&conn).await;
}
