//! In-process HTTP assertions for the read-only search and record pages
//! (ADR 0006, issue 0003 slices S3a-S3b) and the Cmd+K palette's
//! `GET /palette.js` script and `GET /palette` fragment endpoint (ADR 0015,
//! issue 0008 slice S4): a real request/response round trip via
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
    write_record_with_frontmatter(docs, dir, filename, "status: Accepted\n", title, body);
}

fn write_record_with_frontmatter(
    docs: &Path,
    dir: &str,
    filename: &str,
    extra_frontmatter: &str,
    title: &str,
    body: &str,
) {
    let type_dir = docs.join(dir);
    fs::create_dir_all(&type_dir).expect("create record type dir");
    let contents = format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: seeded for web http test\n{extra_frontmatter}---\n\n# {title}\n\n{body}\n"
    );
    fs::write(type_dir.join(filename), contents).expect("write seeded record");
}

async fn request(router: Router, method: Method, uri: &str) -> Response {
    router
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
        .expect("router responds")
}

async fn get(router: Router, uri: &str) -> Response {
    request(router, Method::GET, uri).await
}

async fn get_ok_body(router: Router, uri: &str) -> String {
    let response = get(router, uri).await;
    assert_eq!(response.status(), StatusCode::OK);
    body_text(response).await
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
    write_record_with_frontmatter(
        &docs,
        "adr",
        "0003-superseded-record.md",
        "status: Deprecated\nsuperseded_by: 0004\ntags: [caching]\n",
        "Superseded Record",
        "This document was superseded by a later decision.",
    );
    write_record_with_frontmatter(
        &docs,
        "adr",
        "0004-superseding-record.md",
        "status: Accepted\nsupersedes: 0003\ntags: [caching, performance]\n",
        "Superseding Record",
        "This document supersedes the prior decision.",
    );

    let db_path = temp_dir("db-parent").join("index.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let conn = db_store::connect(&db_url).await.expect("connect");
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

fn main_pane(body: &str) -> &str {
    let start = body.find("<main>").expect("body has a main pane") + "<main>".len();
    let end = body.find("</main>").expect("body has a main pane");
    &body[start..end]
}

fn aside_pane(body: &str) -> &str {
    let start = body.find("<aside>").expect("body has an aside pane") + "<aside>".len();
    let end = body.find("</aside>").expect("body has an aside pane");
    &body[start..end]
}

#[tokio::test]
async fn search_with_a_matching_term_returns_the_record_title_and_path() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/?q=quokka").await;

    let main = main_pane(&body);
    assert!(main.contains("Quokka Caching Strategy"), "got: {body}");
    assert!(main.contains("adr/0001-quokka-caching.md"), "got: {body}");
    assert!(!main.contains("Unrelated Decision"), "got: {body}");
}

#[tokio::test]
async fn search_with_no_matches_renders_an_empty_state_not_an_error() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/?q=zzzznomatch").await;

    assert!(body.to_lowercase().contains("no results"), "got: {body}");
}

#[tokio::test]
async fn search_with_no_query_renders_only_the_form() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/").await;

    let main = main_pane(&body);
    assert!(main.contains("<form"), "got: {body}");
    assert!(!main.contains("Quokka Caching Strategy"), "got: {body}");
}

#[tokio::test]
async fn record_route_with_a_seeded_path_returns_the_rendered_body() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0001-quokka-caching.md").await;

    assert!(body.contains("Quokka Caching Strategy"), "got: {body}");
    assert!(
        body.contains("<p>We adopt an aggressive quokka caching strategy for search results.</p>"),
        "got: {body}"
    );
}

#[tokio::test]
async fn record_route_with_a_missing_path_returns_404_not_500() {
    let router = seeded_router().await;
    let response = get(router, "/record/adr/9999-missing.md").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = body_text(response).await;
    assert!(body.to_lowercase().contains("not found"), "got: {body}");
}

#[tokio::test]
async fn the_search_page_renders_a_nav_tree_grouped_by_doc_type() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/").await;

    assert!(body.contains("<nav"), "got: {body}");
    assert!(body.contains("<h2>ADR</h2>"), "got: {body}");
    assert!(
        body.contains("href=\"/record/adr/0001-quokka-caching.md\""),
        "got: {body}"
    );
}

#[tokio::test]
async fn the_record_page_marks_its_own_nav_entry_as_the_active_page() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0001-quokka-caching.md").await;

    assert!(
        body.contains("href=\"/record/adr/0001-quokka-caching.md\" aria-current=\"page\""),
        "got: {body}"
    );
    assert!(
        !body.contains("href=\"/record/adr/0002-unrelated.md\" aria-current=\"page\""),
        "got: {body}"
    );
}

#[tokio::test]
async fn style_css_is_served_with_the_css_content_type() {
    let router = seeded_router().await;
    let response = get(router, "/style.css").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .expect("content-type header present"),
        "text/css"
    );
}

#[tokio::test]
async fn a_mutating_method_is_rejected_with_method_not_allowed() {
    let router = seeded_router().await;
    let response = request(router, Method::POST, "/").await;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn the_superseded_records_page_shows_its_status_badge_and_links_to_the_superseding_record() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0003-superseded-record.md").await;

    let aside = aside_pane(&body);
    assert!(
        aside.contains("class=\"status-badge status-deprecated\""),
        "got: {body}"
    );
    assert!(aside.contains(">Deprecated<"), "got: {body}");
    assert!(
        aside.contains("href=\"/record/adr/0004-superseding-record.md\""),
        "got: {body}"
    );
    assert!(aside.contains("Superseding Record"), "got: {body}");
}

#[tokio::test]
async fn the_superseding_records_page_links_back_under_supersedes() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0004-superseding-record.md").await;

    let aside = aside_pane(&body);
    assert!(aside.contains("Supersedes"), "got: {body}");
    assert!(
        aside.contains("href=\"/record/adr/0003-superseded-record.md\""),
        "got: {body}"
    );
    assert!(aside.contains("Superseded Record"), "got: {body}");
}

#[tokio::test]
async fn a_record_with_tags_shows_its_tag_chips() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0004-superseding-record.md").await;

    let aside = aside_pane(&body);
    assert!(
        aside.contains("<span class=\"tag\">caching</span>"),
        "got: {body}"
    );
    assert!(
        aside.contains("<span class=\"tag\">performance</span>"),
        "got: {body}"
    );
}

#[tokio::test]
async fn a_record_page_renders_exactly_one_h1_within_main() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0001-quokka-caching.md").await;

    let main = main_pane(&body);
    assert_eq!(main.matches("<h1").count(), 1, "got: {body}");
}

#[tokio::test]
async fn sections_for_empty_relation_and_tag_lists_are_absent() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/record/adr/0002-unrelated.md").await;

    let aside = aside_pane(&body);
    assert!(!aside.contains("Supersedes"), "got: {body}");
    assert!(!aside.contains("Superseded by"), "got: {body}");
    assert!(!aside.contains("class=\"tag\""), "got: {body}");
}

#[tokio::test]
async fn palette_js_is_served_with_a_javascript_content_type_and_a_non_empty_body() {
    let router = seeded_router().await;
    let response = get(router, "/palette.js").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .expect("content-type header present"),
        "application/javascript"
    );
    let body = body_text(response).await;
    assert!(!body.trim().is_empty(), "expected a non-empty script body");
}

#[tokio::test]
async fn palette_with_a_matching_term_returns_a_bare_fragment_linking_to_the_record() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/palette?q=quokka").await;

    assert!(
        body.contains("href=\"/record/adr/0001-quokka-caching.md\""),
        "got: {body}"
    );
    assert!(body.contains("Quokka Caching Strategy"), "got: {body}");
    assert!(!body.contains("<nav"), "got: {body}");
    assert!(!body.contains("<html"), "got: {body}");
}

#[tokio::test]
async fn palette_with_a_blank_query_returns_the_empty_state_fragment() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/palette?q=").await;

    assert!(body.contains("empty-state"), "got: {body}");
    assert!(!body.contains("<nav"), "got: {body}");
}

#[tokio::test]
async fn palette_with_no_query_returns_the_empty_state_fragment() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/palette").await;

    assert!(body.contains("empty-state"), "got: {body}");
}

#[tokio::test]
async fn the_home_page_loads_the_deferred_palette_script_and_the_hidden_overlay() {
    let router = seeded_router().await;
    let body = get_ok_body(router, "/").await;

    assert!(
        body.contains("<script src=\"/palette.js\" defer></script>"),
        "got: {body}"
    );
    assert!(
        body.contains("id=\"palette-overlay\" hidden"),
        "got: {body}"
    );
}
