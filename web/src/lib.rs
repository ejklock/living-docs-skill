//! Library surface shared between the `living-docs-web` binary and its
//! integration tests (ADR 0006, issue 0003 slices S3a-S3b): the read-only
//! axum router, its `GET /` search handler, and its `GET /record/{*path}`
//! record handler.

pub mod views;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use db_store::{ProjectView, RecordView, SearchHit};
use maud::Markup;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

/// Query-string parameters accepted by the search page. `project`, when
/// present and non-empty, narrows the search to one project's slug (ADR
/// 0005, issue 0005 slice 0005-C2); omitted or blank spans every project.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    project: Option<String>,
}

/// Builds the read-only router backed by `conn`. Exposes `GET /` and
/// `GET /record/{*path}` only — no route here mutates the read-model.
pub fn build_router(conn: DatabaseConnection) -> Router {
    Router::new()
        .route("/", get(search_handler))
        .route("/record/{*path}", get(record_handler))
        .with_state(conn)
}

async fn search_handler(
    State(conn): State<DatabaseConnection>,
    Query(params): Query<SearchParams>,
) -> Markup {
    let query = params.q.filter(|value| !value.trim().is_empty());
    let project = params.project.filter(|value| !value.trim().is_empty());
    let hits = match &query {
        Some(term) => search_or_log(&conn, term, project.as_deref()).await,
        None => Vec::new(),
    };
    let projects = list_projects_or_log(&conn).await;
    views::search_page(query.as_deref(), project.as_deref(), &projects, &hits)
}

async fn search_or_log(
    conn: &DatabaseConnection,
    query: &str,
    project: Option<&str>,
) -> Vec<SearchHit> {
    let result = match project {
        Some(slug) => db_store::search_in_project(conn, query, slug).await,
        None => db_store::search(conn, query).await,
    };
    match result {
        Ok(hits) => hits,
        Err(err) => {
            eprintln!("search query {query:?} (project: {project:?}) failed: {err}");
            Vec::new()
        }
    }
}

async fn list_projects_or_log(conn: &DatabaseConnection) -> Vec<ProjectView> {
    match db_store::list_projects(conn).await {
        Ok(projects) => projects,
        Err(err) => {
            eprintln!("listing projects failed: {err}");
            Vec::new()
        }
    }
}

async fn record_handler(
    State(conn): State<DatabaseConnection>,
    Path(path): Path<String>,
) -> Response {
    match record_or_log(&conn, &path).await {
        Some(record) => {
            let body_html = render_markdown(&record.body);
            (
                StatusCode::OK,
                views::record_page(&record.title, &body_html),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, views::not_found()).into_response(),
    }
}

async fn record_or_log(conn: &DatabaseConnection, path: &str) -> Option<RecordView> {
    match db_store::record_by_path(conn, path).await {
        Ok(record) => record,
        Err(err) => {
            eprintln!("record lookup {path:?} failed: {err}");
            None
        }
    }
}

fn render_markdown(body: &str) -> String {
    let parser = pulldown_cmark::Parser::new(body);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}
