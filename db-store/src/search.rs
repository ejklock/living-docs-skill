//! Ranked full-text search over the backend-native index (ADR 0004, issue
//! 0002 slice S2b; ParadeDB BM25 branch issue 0004 slice 0004-C;
//! cross-project + project-scoped search ADR 0005 issue 0005 slice
//! 0005-C1).

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, FromQueryResult, Statement};

use crate::record::SearchHit;
use crate::Result;

#[derive(Debug, FromQueryResult)]
struct SearchRow {
    path: String,
    title: String,
    snippet: String,
    project: String,
}

/// Runs a ranked full-text query across every project, returning hits
/// best-match-first and labeling each with its project slug. A query
/// matching no record returns an empty vector. Equivalent to
/// `search_in_project` with no scope; see [`search_in_project`] to narrow
/// to one project.
pub async fn search(conn: &DatabaseConnection, query: &str) -> Result<Vec<SearchHit>> {
    run_search(conn, query, None).await
}

/// Runs the same ranked full-text query as [`search`], narrowed to records
/// belonging to `project_slug` only (ADR 0005, issue 0005 slice 0005-C1).
pub async fn search_in_project(
    conn: &DatabaseConnection,
    query: &str,
    project_slug: &str,
) -> Result<Vec<SearchHit>> {
    run_search(conn, query, Some(project_slug)).await
}

/// Shared implementation behind [`search`] and [`search_in_project`]:
/// SQLite matches via FTS5's `MATCH` operator over `records_fts`; Postgres
/// matches via `pg_search`'s BM25 `@@@` operator over the `records_bm25`
/// index. Both branches join `projects` to label every hit and, when
/// `scope` is `Some`, add a project-slug predicate.
async fn run_search(
    conn: &DatabaseConnection,
    query: &str,
    scope: Option<&str>,
) -> Result<Vec<SearchHit>> {
    let rows = match conn.get_database_backend() {
        DbBackend::Sqlite => sqlite_search(conn, query, scope).await?,
        DbBackend::Postgres => postgres_search(conn, query, scope).await?,
        DbBackend::MySql => return Err(unsupported_backend_err()),
    };

    Ok(rows.into_iter().map(row_to_hit).collect())
}

/// Projects a raw `SearchRow` (shared by both the SQLite and Postgres
/// branches) into the public `SearchHit` shape.
fn row_to_hit(row: SearchRow) -> SearchHit {
    SearchHit {
        path: row.path,
        title: row.title,
        snippet: row.snippet,
        project: row.project,
    }
}

async fn sqlite_search(
    conn: &DatabaseConnection,
    query: &str,
    scope: Option<&str>,
) -> Result<Vec<SearchRow>> {
    let base = "SELECT r.path AS path, r.title AS title, p.slug AS project, \
                snippet(records_fts, 2, '[', ']', '\u{2026}', 10) AS snippet \
                FROM records_fts \
                JOIN records r ON r.id = records_fts.rowid \
                JOIN projects p ON p.id = r.project_id \
                WHERE records_fts MATCH ?1";
    let statement = match scope {
        Some(slug) => Statement::from_sql_and_values(
            conn.get_database_backend(),
            format!("{base} AND p.slug = ?2 ORDER BY rank"),
            [query.into(), slug.into()],
        ),
        None => Statement::from_sql_and_values(
            conn.get_database_backend(),
            format!("{base} ORDER BY rank"),
            [query.into()],
        ),
    };

    SearchRow::find_by_statement(statement).all(conn).await
}

async fn postgres_search(
    conn: &DatabaseConnection,
    query: &str,
    scope: Option<&str>,
) -> Result<Vec<SearchRow>> {
    let base = "SELECT r.path AS path, r.title AS title, p.slug AS project, \
                paradedb.snippet(r.body) AS snippet \
                FROM records r \
                JOIN projects p ON p.id = r.project_id \
                WHERE r.id @@@ paradedb.boolean(should => ARRAY[\
                paradedb.match('title', $1), \
                paradedb.match('body', $1), \
                paradedb.match('description', $1)\
                ])";
    let statement = match scope {
        Some(slug) => Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("{base} AND p.slug = $2 ORDER BY paradedb.score(r.id) DESC"),
            [query.into(), slug.into()],
        ),
        None => Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("{base} ORDER BY paradedb.score(r.id) DESC"),
            [query.into()],
        ),
    };

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
