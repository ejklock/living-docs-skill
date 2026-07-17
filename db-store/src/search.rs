//! Ranked full-text search over the backend-native index (ADR 0004, issue
//! 0002 slice S2b; ParadeDB BM25 branch issue 0004 slice 0004-C).

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, FromQueryResult, Statement};

use crate::record::SearchHit;
use crate::Result;

#[derive(Debug, FromQueryResult)]
struct SearchRow {
    path: String,
    title: String,
    snippet: String,
}

/// Runs a ranked full-text query against the backend-native search index,
/// returning hits best-match-first. A query matching no record returns an
/// empty vector. SQLite matches via FTS5's `MATCH` operator over
/// `records_fts`; Postgres matches via `pg_search`'s BM25 `@@@` operator over
/// the `records_bm25` index.
pub async fn search(conn: &DatabaseConnection, query: &str) -> Result<Vec<SearchHit>> {
    let rows = match conn.get_database_backend() {
        DbBackend::Sqlite => sqlite_search(conn, query).await?,
        DbBackend::Postgres => postgres_search(conn, query).await?,
        DbBackend::MySql => return Err(unsupported_backend_err()),
    };

    Ok(rows
        .into_iter()
        .map(|row| SearchHit {
            path: row.path,
            title: row.title,
            snippet: row.snippet,
        })
        .collect())
}

async fn sqlite_search(conn: &DatabaseConnection, query: &str) -> Result<Vec<SearchRow>> {
    let statement = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT r.path AS path, r.title AS title, \
         snippet(records_fts, 2, '[', ']', '\u{2026}', 10) AS snippet \
         FROM records_fts \
         JOIN records r ON r.id = records_fts.rowid \
         WHERE records_fts MATCH ?1 \
         ORDER BY rank",
        [query.into()],
    );

    SearchRow::find_by_statement(statement).all(conn).await
}

async fn postgres_search(conn: &DatabaseConnection, query: &str) -> Result<Vec<SearchRow>> {
    let statement = Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT path, title, paradedb.snippet(body) AS snippet \
         FROM records \
         WHERE id @@@ paradedb.boolean(should => ARRAY[\
         paradedb.match('title', $1), \
         paradedb.match('body', $1), \
         paradedb.match('description', $1)\
         ]) \
         ORDER BY paradedb.score(id) DESC",
        [query.into()],
    );

    SearchRow::find_by_statement(statement).all(conn).await
}

/// The error returned when a search runs against a backend this crate does
/// not support (only SQLite and Postgres are compiled in).
fn unsupported_backend_err() -> DbErr {
    DbErr::Custom("db-store only supports the sqlite and postgres backends".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::sync;
    use crate::sync::test_support::seeded_corpus;
    use crate::{connect_in_memory, migrate};

    #[tokio::test]
    async fn search_ranks_the_matching_record_first() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = seeded_corpus();
        sync(&conn, &store, &bundle).await.expect("sync");

        let hits = search(&conn, "quokka").await.expect("search");

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "adr/0001-quokka-caching.md");
        assert_eq!(hits[0].title, "Quokka Caching Strategy");
        assert!(hits[0].snippet.contains('['));
    }

    #[tokio::test]
    async fn search_with_no_match_returns_an_empty_result() {
        let conn = connect_in_memory().await.expect("connect");
        migrate(&conn).await.expect("migrate");
        let (store, bundle) = seeded_corpus();
        sync(&conn, &store, &bundle).await.expect("sync");

        let hits = search(&conn, "zzzznomatch").await.expect("search");

        assert!(hits.is_empty());
    }
}
