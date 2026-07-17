//! Ranked full-text search over the `records_fts` index (ADR 0004, issue
//! 0002 slice S2b).

use sea_orm::{ConnectionTrait, DatabaseConnection, FromQueryResult, Statement};

use crate::record::SearchHit;
use crate::Result;

#[derive(Debug, FromQueryResult)]
struct SearchRow {
    path: String,
    title: String,
    snippet: String,
}

/// Runs an FTS5 `MATCH` query against `records_fts`, returning hits ranked
/// best-match-first. A query matching no record returns an empty vector.
pub async fn search(conn: &DatabaseConnection, query: &str) -> Result<Vec<SearchHit>> {
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

    let rows = SearchRow::find_by_statement(statement).all(conn).await?;
    Ok(rows
        .into_iter()
        .map(|row| SearchHit {
            path: row.path,
            title: row.title,
            snippet: row.snippet,
        })
        .collect())
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
