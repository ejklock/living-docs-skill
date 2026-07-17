//! Library surface shared between the `living-docs-web` binary and its
//! integration tests (ADR 0006, issue 0003 slice S3a): the read-only axum
//! router and its `GET /` search handler.

pub mod views;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::Router;
use db_store::SearchHit;
use maud::Markup;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

/// Query-string parameters accepted by the search page.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    q: Option<String>,
}

/// Builds the read-only router backed by `conn`. Exposes `GET /` only — no
/// route here mutates the read-model.
pub fn build_router(conn: DatabaseConnection) -> Router {
    Router::new()
        .route("/", get(search_handler))
        .with_state(conn)
}

async fn search_handler(
    State(conn): State<DatabaseConnection>,
    Query(params): Query<SearchParams>,
) -> Markup {
    let query = params.q.filter(|value| !value.trim().is_empty());
    let hits = match &query {
        Some(term) => search_or_log(&conn, term).await,
        None => Vec::new(),
    };
    views::search_page(query.as_deref(), &hits)
}

async fn search_or_log(conn: &DatabaseConnection, query: &str) -> Vec<SearchHit> {
    match db_store::search(conn, query).await {
        Ok(hits) => hits,
        Err(err) => {
            eprintln!("search query {query:?} failed: {err}");
            Vec::new()
        }
    }
}
