//! Library surface shared between the `living-docs-web` binary and its
//! integration tests (ADR 0006, issue 0003 slices S3a-S3b; three-pane shell
//! ADR 0015, issue 0008 slices S2-S4): the read-only axum router, its
//! `GET /` search handler, its `GET /record/{*path}` record handler
//! (metadata panel fed by `db_store::record_meta`, S3), its
//! `GET /style.css` stylesheet route, and the Cmd+K palette's
//! `GET /palette.js` static script plus its `GET /palette` fragment
//! endpoint (S4) reusing the same async `db_store` search path as `GET /`.

pub mod views;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use db_store::{NavEntry, ProjectView, RecordMeta, RecordView, SearchHit};
use maud::Markup;
use sea_orm::DatabaseConnection;
use serde::Deserialize;

const STYLESHEET: &str = include_str!("style.css");
const PALETTE_SCRIPT: &str = include_str!("palette.js");

/// Query-string parameters accepted by the search page. `project`, when
/// present and non-empty, narrows the search to one project's slug (ADR
/// 0005, issue 0005 slice 0005-C2); omitted or blank spans every project.
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    q: Option<String>,
    project: Option<String>,
}

/// Builds the read-only router backed by `conn`. Exposes `GET /`,
/// `GET /record/{*path}`, `GET /style.css`, `GET /palette.js`, and
/// `GET /palette` only — no route here mutates the read-model.
pub fn build_router(conn: DatabaseConnection) -> Router {
    Router::new()
        .route("/", get(search_handler))
        .route("/record/{*path}", get(record_handler))
        .route("/style.css", get(style_handler))
        .route("/palette.js", get(palette_script_handler))
        .route("/palette", get(palette_handler))
        .with_state(conn)
}

async fn style_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], STYLESHEET)
}

async fn palette_script_handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        PALETTE_SCRIPT,
    )
}

async fn palette_handler(
    State(conn): State<DatabaseConnection>,
    Query(params): Query<SearchParams>,
) -> Markup {
    let query = params.q.filter(|value| !value.trim().is_empty());
    let hits = match &query {
        Some(term) => search_or_log(&conn, term, None).await,
        None => Vec::new(),
    };
    views::palette_fragment(&hits)
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
    let nav = nav_entries_or_log(&conn).await;
    let main = views::search_page(query.as_deref(), project.as_deref(), &projects, &hits);
    views::shell("living-docs search", &nav, None, main, None)
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

async fn nav_entries_or_log(conn: &DatabaseConnection) -> Vec<NavEntry> {
    match db_store::records_by_type(conn).await {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("listing nav entries failed: {err}");
            Vec::new()
        }
    }
}

async fn record_handler(
    State(conn): State<DatabaseConnection>,
    Path(path): Path<String>,
) -> Response {
    let nav = nav_entries_or_log(&conn).await;
    match record_or_log(&conn, &path).await {
        Some(record) => {
            let body_html = render_markdown(&record.body);
            let title = format!("{} — living-docs", record.title);
            let main = views::record_page(&body_html);
            let aside = record_meta_or_log(&conn, &path)
                .await
                .map(|meta| views::metadata_panel(&meta));
            let shell = views::shell(&title, &nav, Some(path.as_str()), main, aside);
            (StatusCode::OK, shell).into_response()
        }
        None => {
            let shell = views::shell(
                "Not found — living-docs",
                &nav,
                None,
                views::not_found(),
                None,
            );
            (StatusCode::NOT_FOUND, shell).into_response()
        }
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

async fn record_meta_or_log(conn: &DatabaseConnection, path: &str) -> Option<RecordMeta> {
    match db_store::record_meta(conn, path).await {
        Ok(meta) => meta,
        Err(err) => {
            eprintln!("record_meta lookup {path:?} failed: {err}");
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
