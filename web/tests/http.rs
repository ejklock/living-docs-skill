//! In-process HTTP assertions for the read-only search page (ADR 0006, issue
//! 0003 slice S3a): a real request/response round trip via
//! `tower::ServiceExt::oneshot`, with no bound TCP port.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("living-docs-web-http-test-{label}-{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_record(docs: &Path, dir: &str, filename: &str, title: &str, body: &str) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).expect("create record type dir");
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: seeded for web http test\nstatus: Accepted\n---\n\n# {title}\n\n{body}\n"
    );
    fs::write(type_dir.join(filename), contents).expect("write seeded record");
}

async fn seeded_router() -> Router {
    let docs = temp_dir("docs");
    write_record(
        &docs,
        "adr",
        "0001-quokka-caching.md",
        "Quokka Caching Strategy",
        "We adopt an aggressive quokka caching strategy for search results.",
    );
    write_record(
        &docs,
        "adr",
        "0002-unrelated.md",
        "Unrelated Decision",
        "This document discusses logging conventions.",
    );

    let db_path = temp_dir("db-parent").join("index.db");
    let conn = db_store::connect(&db_path).await.expect("connect");
    db_store::migrate(&conn).await.expect("migrate");
    db_store::sync(&conn, &fs_store::FsStore::new(), &docs)
        .await
        .expect("sync seeded corpus");

    web::build_router(conn)
}

async fn body_text(response: Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect response body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("response body is valid utf-8")
}

#[tokio::test]
async fn search_with_a_matching_term_returns_the_record_title_and_path() {
    let router = seeded_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/?q=quokka")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("Quokka Caching Strategy"), "got: {body}");
    assert!(body.contains("adr/0001-quokka-caching.md"), "got: {body}");
    assert!(!body.contains("Unrelated Decision"), "got: {body}");
}

#[tokio::test]
async fn search_with_no_matches_renders_an_empty_state_not_an_error() {
    let router = seeded_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/?q=zzzznomatch")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.to_lowercase().contains("no results"), "got: {body}");
}

#[tokio::test]
async fn search_with_no_query_renders_only_the_form() {
    let router = seeded_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("<form"), "got: {body}");
    assert!(!body.contains("Quokka Caching Strategy"), "got: {body}");
}

#[tokio::test]
async fn a_mutating_method_is_rejected_with_method_not_allowed() {
    let router = seeded_router().await;

    let response = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/")
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router responds");

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
