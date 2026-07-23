//! In-process HTTP assertions for the read-only search and record pages
//! (ADR 0006, issue 0003 slices S3a-S3b), the Cmd+K palette's
//! `GET /palette.js` script and `GET /palette` fragment endpoint (ADR 0015,
//! issue 0008 slice S4), and Atlas's authoring create route (ADR 0016,
//! issue 0010 slice 3): a real request/response round trip via
//! `tower::ServiceExt::oneshot`, with no bound TCP port.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// A per-process counter appended to every [`temp_dir`], on top of the
/// nanosecond timestamp: this suite's tests run concurrently and this file
/// now calls `authoring_fixture` (and so `temp_dir`) often enough that two
/// threads can land on the same OS clock tick on a coarser-resolution
/// platform, colliding on one sqlite file and failing a totally unrelated
/// test with a spurious "table already exists". The counter makes every
/// call unique regardless of clock resolution.
static TEMP_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "living-docs-web-http-test-{label}-{nanos}-{sequence}"
    ));
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

    web::build_router(conn, None)
}

async fn post_form(router: Router, uri: &str, form_body: &str) -> Response {
    router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form_body.to_owned()))
                .expect("build request"),
        )
        .await
        .expect("router responds")
}

/// A minimal on-disk check-passing skeleton mirroring
/// `cli/tests/db_authoring.rs`'s own fixture: a bundle-root `index.md`
/// linking to `adr/index.md`, which in turn lists `adr_entries` —
/// `write_checked`'s in-transaction `check` reads these `index.md` files
/// straight off disk regardless of backend, so a slug must already be
/// listed here before a form submission targeting it can commit.
fn seed_index_skeleton(docs: &Path, adr_entries: &[&str]) {
    fs::create_dir_all(docs.join("adr")).expect("create adr dir");
    fs::write(docs.join("index.md"), "# Index\n\n- [ADRs](adr/index.md)\n")
        .expect("write root index");
    let rows: String = adr_entries
        .iter()
        .map(|entry| format!("- [{entry}]({entry}.md)\n"))
        .collect();
    fs::write(
        docs.join("adr").join("index.md"),
        format!("# ADRs\n\n{rows}"),
    )
    .expect("write adr index");
}

/// The ADR template's own "fill this in" example links
/// (`research/NNNN-<slug>.md`, `prd/NNNN-<slug>.md`, and the References
/// section's literal `adr/url`) resolve to real paths on disk here, so a
/// freshly planned record's `check` gate has something to find — mirrors
/// `cli/tests/db_authoring.rs`'s `seed_adr_placeholder_link_targets`
/// exactly, since `create_handler` fills the very same embedded template.
fn seed_adr_placeholder_link_targets(docs: &Path) {
    fs::create_dir_all(docs.join("research").join("NNNN-<slug>.md"))
        .expect("seed research placeholder dir");
    fs::create_dir_all(docs.join("prd").join("NNNN-<slug>.md")).expect("seed prd placeholder dir");
    fs::create_dir_all(docs.join("adr")).expect("create adr dir");
    fs::write(docs.join("adr").join("url"), "").expect("seed adr url placeholder");
}

/// Builds a db-mode-authoring router: a temp sqlite db migrated and synced
/// from a docs bundle that already carries a starter record (so
/// `DbDocStore::new`'s project-must-exist precondition holds), the
/// `index.md` skeleton, and the ADR template's placeholder link targets —
/// pre-listing exactly the slugs `adr_entries` names as already anticipated,
/// the same precondition `db_store::DbDocStore::write_checked`'s own tests
/// rely on.
async fn authoring_fixture(adr_entries: &[&str]) -> Router {
    let docs = temp_dir("docs-authoring");
    seed_index_skeleton(&docs, adr_entries);
    seed_adr_placeholder_link_targets(&docs);
    write_record(
        &docs,
        "adr",
        "0001-starter-record.md",
        "Starter Record",
        "A starter record so the default project already exists before authoring.",
    );

    let db_path = temp_dir("db-authoring-parent").join("index.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let conn = db_store::connect(&db_url).await.expect("connect");
    db_store::migrate(&conn).await.expect("migrate");
    db_store::sync(&conn, &fs_store::FsStore::new(), &docs)
        .await
        .expect("sync seeded corpus");

    let authoring = web::AuthoringConfig {
        db_url,
        docs_root: docs,
    };
    web::build_router(conn, Some(authoring))
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

#[tokio::test]
async fn new_route_is_unregistered_in_file_mode_returning_404_for_get_and_post() {
    let router = seeded_router().await;

    let get_response = get(router.clone(), "/new").await;
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);

    let post_response = request(router, Method::POST, "/new").await;
    assert_eq!(post_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn creating_a_record_in_db_mode_persists_it_and_redirects_to_its_record_page() {
    let router = authoring_fixture(&["0001-starter-record", "0002-new-feature"]).await;

    let response = post_form(router.clone(), "/new", "doc_type=adr&title=New+Feature").await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .expect("redirect carries a Location header")
        .to_str()
        .expect("location header is valid utf-8")
        .to_owned();
    assert_eq!(location, "/record/adr/0002-new-feature.md");

    let body = get_ok_body(router, &location).await;
    assert!(body.contains("New Feature"), "got: {body}");
}

#[tokio::test]
async fn a_submission_whose_content_fails_the_check_gate_re_renders_the_form_with_an_error_and_commits_nothing(
) {
    let router = authoring_fixture(&["0001-starter-record", "0002-new-feature"]).await;

    let first = post_form(router.clone(), "/new", "doc_type=adr&title=New+Feature").await;
    assert_eq!(first.status(), StatusCode::SEE_OTHER);

    let second = post_form(router.clone(), "/new", "doc_type=issue&title=Some+Issue").await;
    assert_eq!(second.status(), StatusCode::OK);
    let body = body_text(second).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(body.contains("value=\"Some Issue\""), "got: {body}");

    let index_body = get_ok_body(router, "/").await;
    assert!(
        !index_body.contains("Some Issue"),
        "the rejected issue submission must not have committed a record: {index_body}"
    );
}

fn edit_form_body(content: &str, base_revision: i64) -> String {
    format!(
        "content={}&base_revision={base_revision}",
        urlencoding_percent_encode(content)
    )
}

fn urlencoding_percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

const EDITABLE_RECORD_BODY: &str =
    "---\ntype: ADR\ntitle: Starter Record\ndescription: seeded for web http test\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Starter Record\n\nOriginal body.\n";

const UPDATED_RECORD_BODY: &str =
    "---\ntype: ADR\ntitle: Starter Record\ndescription: seeded for web http test\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Starter Record\n\nEdited body.\n";

const CONFLICTING_RECORD_BODY: &str =
    "---\ntype: ADR\ntitle: Starter Record\ndescription: seeded for web http test\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Starter Record\n\nA conflicting edit that must never land.\n";

const BROKEN_RECORD_BODY: &str =
    "---\ntype: ADR\ntitle: Starter Record\ndescription: seeded for web http test\nstatus: Superseded\nsuperseded_by: 9999\ntags: []\ntimestamp: 2026-07-21T00:00:00Z\n---\n\n# Starter Record\n\nBroken.\n";

/// An `authoring_fixture` whose starter record's own canonical markdown is
/// known (needed to build a valid, check-passing edit body against it) — an
/// edit route needs an existing record to edit, unlike the create route's
/// tests, which always target a brand-new path.
async fn authoring_fixture_with_editable_starter() -> Router {
    authoring_fixture(&["0001-starter-record"]).await
}

#[tokio::test]
async fn a_valid_edit_commits_bumps_revision_and_the_record_page_reflects_the_new_content() {
    let router = authoring_fixture_with_editable_starter().await;

    let form = get_ok_body(router.clone(), "/edit/adr/0001-starter-record.md").await;
    assert!(
        form.contains("name=\"base_revision\" value=\"1\""),
        "got: {form}"
    );

    let response = post_form(
        router.clone(),
        "/edit/adr/0001-starter-record.md",
        &edit_form_body(UPDATED_RECORD_BODY, 1),
    )
    .await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .expect("redirect carries a Location header")
        .to_str()
        .expect("location header is valid utf-8")
        .to_owned();
    assert_eq!(location, "/record/adr/0001-starter-record.md");

    let record_body = get_ok_body(router.clone(), &location).await;
    assert!(record_body.contains("Edited body."), "got: {record_body}");

    let reloaded_form = get_ok_body(router, "/edit/adr/0001-starter-record.md").await;
    assert!(
        reloaded_form.contains("name=\"base_revision\" value=\"2\""),
        "got: {reloaded_form}"
    );
}

#[tokio::test]
async fn an_edit_whose_content_fails_the_check_gate_re_renders_the_form_with_an_error_and_leaves_the_record_unchanged(
) {
    let router = authoring_fixture_with_editable_starter().await;

    let response = post_form(
        router.clone(),
        "/edit/adr/0001-starter-record.md",
        &edit_form_body(BROKEN_RECORD_BODY, 1),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(body.contains("Broken."), "got: {body}");

    let record_body = get_ok_body(router, "/record/adr/0001-starter-record.md").await;
    assert!(
        !record_body.contains("Broken."),
        "a rejected edit must not have committed: {record_body}"
    );
}

#[tokio::test]
async fn a_stale_base_revision_is_rejected_and_the_form_reloads_the_current_server_content_not_the_second_submission(
) {
    let router = authoring_fixture_with_editable_starter().await;

    let first = post_form(
        router.clone(),
        "/edit/adr/0001-starter-record.md",
        &edit_form_body(UPDATED_RECORD_BODY, 1),
    )
    .await;
    assert_eq!(first.status(), StatusCode::SEE_OTHER);

    let second = post_form(
        router.clone(),
        "/edit/adr/0001-starter-record.md",
        &edit_form_body(CONFLICTING_RECORD_BODY, 1),
    )
    .await;

    assert_eq!(second.status(), StatusCode::OK);
    let body = body_text(second).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(
        body.contains("Edited body."),
        "the reloaded form must show the current (first edit's) server content: {body}"
    );
    assert!(
        !body.contains("A conflicting edit"),
        "the reloaded form must never show the rejected second submission: {body}"
    );
    assert!(
        body.contains("name=\"base_revision\" value=\"2\""),
        "the reloaded form must carry the current server revision: {body}"
    );

    let record_body = get_ok_body(router, "/record/adr/0001-starter-record.md").await;
    assert!(
        record_body.contains("Edited body."),
        "the stored record must remain exactly what the first commit left it as: {record_body}"
    );
    assert!(
        !record_body.contains("A conflicting edit"),
        "got: {record_body}"
    );
}

#[tokio::test]
async fn get_edit_on_a_path_with_no_existing_record_returns_404() {
    let router = authoring_fixture_with_editable_starter().await;

    let get_response = get(router, "/edit/adr/9999-missing.md").await;

    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
    let body = body_text(get_response).await;
    assert!(body.to_lowercase().contains("not found"), "got: {body}");
}

#[tokio::test]
async fn post_edit_on_a_path_with_no_existing_record_re_renders_the_form_with_a_not_found_error() {
    let router = authoring_fixture_with_editable_starter().await;

    let post_response = post_form(
        router,
        "/edit/adr/9999-missing.md",
        &edit_form_body(EDITABLE_RECORD_BODY, 1),
    )
    .await;

    assert_eq!(post_response.status(), StatusCode::OK);
    let body = body_text(post_response).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(
        body.to_lowercase().contains("no record at that path"),
        "got: {body}"
    );
}

#[tokio::test]
async fn edit_routes_are_unregistered_in_file_mode_returning_404_for_get_and_post() {
    let router = seeded_router().await;

    let get_response = get(router.clone(), "/edit/adr/0001-quokka-caching.md").await;
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);

    let post_response = post_form(
        router,
        "/edit/adr/0001-quokka-caching.md",
        &edit_form_body(EDITABLE_RECORD_BODY, 1),
    )
    .await;
    assert_eq!(post_response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_record_page_renders_an_edit_link_in_db_mode_authoring() {
    let router = authoring_fixture_with_editable_starter().await;

    let body = get_ok_body(router, "/record/adr/0001-starter-record.md").await;

    assert!(
        body.contains("href=\"/edit/adr/0001-starter-record.md\" class=\"edit-link\""),
        "got: {body}"
    );
}

#[tokio::test]
async fn the_record_page_renders_no_edit_link_in_file_mode() {
    let router = seeded_router().await;

    let body = get_ok_body(router, "/record/adr/0001-quokka-caching.md").await;

    assert!(!body.contains("class=\"edit-link\""), "got: {body}");
}

/// An `authoring_fixture`-style router seeded with TWO starter ADR records
/// (`0001-old-record.md`/`0002-new-record.md`) rather than
/// `authoring_fixture`'s single starter — a supersede route needs two
/// existing records to link, unlike create/edit's tests, which each need
/// only one. `extra_records` seeds any additional records (slug, extra
/// frontmatter, title, body) before the sync, e.g. an already
/// check-failing record for the check-gate-failure test.
async fn supersede_fixture(extra_records: &[(&str, &str, &str, &str)]) -> Router {
    let docs = temp_dir("docs-authoring-supersede");
    let mut adr_entries = vec!["0001-old-record", "0002-new-record"];
    adr_entries.extend(extra_records.iter().map(|(slug, ..)| *slug));
    seed_index_skeleton(&docs, &adr_entries);
    seed_adr_placeholder_link_targets(&docs);
    write_record(
        &docs,
        "adr",
        "0001-old-record.md",
        "Old Record",
        "The record a supersede route targets as its old side.",
    );
    write_record(
        &docs,
        "adr",
        "0002-new-record.md",
        "New Record",
        "The record a supersede route targets as its new side.",
    );
    for (slug, extra_frontmatter, title, body) in extra_records {
        write_record_with_frontmatter(
            &docs,
            "adr",
            &format!("{slug}.md"),
            extra_frontmatter,
            title,
            body,
        );
    }

    let db_path = temp_dir("db-authoring-supersede-parent").join("index.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let conn = db_store::connect(&db_url).await.expect("connect");
    db_store::migrate(&conn).await.expect("migrate");
    db_store::sync(&conn, &fs_store::FsStore::new(), &docs)
        .await
        .expect("sync seeded corpus");

    let authoring = web::AuthoringConfig {
        db_url,
        docs_root: docs,
    };
    web::build_router(conn, Some(authoring))
}

#[tokio::test]
async fn superseding_a_record_in_db_mode_commits_bumps_both_revisions_and_both_pages_reflect_the_new_chain(
) {
    let router = supersede_fixture(&[]).await;

    let response = post_form(router.clone(), "/supersede/adr/0001-old-record.md", "new=2").await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .expect("redirect carries a Location header")
        .to_str()
        .expect("location header is valid utf-8")
        .to_owned();
    assert_eq!(location, "/record/adr/0001-old-record.md");

    let old_body = get_ok_body(router.clone(), &location).await;
    let old_aside = aside_pane(&old_body);
    assert!(
        old_aside.contains("class=\"status-badge status-superseded\""),
        "got: {old_body}"
    );
    assert!(
        old_aside.contains("href=\"/record/adr/0002-new-record.md\""),
        "got: {old_body}"
    );

    let new_body = get_ok_body(router.clone(), "/record/adr/0002-new-record.md").await;
    let new_aside = aside_pane(&new_body);
    assert!(new_aside.contains("Supersedes"), "got: {new_body}");
    assert!(
        new_aside.contains("href=\"/record/adr/0001-old-record.md\""),
        "got: {new_body}"
    );

    let old_edit_form = get_ok_body(router.clone(), "/edit/adr/0001-old-record.md").await;
    assert!(
        old_edit_form.contains("name=\"base_revision\" value=\"2\""),
        "got: {old_edit_form}"
    );
    let new_edit_form = get_ok_body(router, "/edit/adr/0002-new-record.md").await;
    assert!(
        new_edit_form.contains("name=\"base_revision\" value=\"2\""),
        "got: {new_edit_form}"
    );
}

#[tokio::test]
async fn superseding_with_a_nonexistent_target_number_re_renders_the_old_records_page_with_the_cli_error_and_commits_nothing(
) {
    let router = supersede_fixture(&[]).await;

    let response = post_form(
        router.clone(),
        "/supersede/adr/0001-old-record.md",
        "new=99",
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(
        body.to_lowercase().contains("no record found for 0099"),
        "got: {body}"
    );
    assert!(
        body.contains("value=\"99\""),
        "the rejected submission's own value must be preserved: got: {body}"
    );

    let old_body = get_ok_body(router.clone(), "/record/adr/0001-old-record.md").await;
    let old_aside = aside_pane(&old_body);
    assert!(
        !old_aside.contains("Superseded"),
        "a resolution failure must not commit: got: {old_body}"
    );

    let old_edit_form = get_ok_body(router, "/edit/adr/0001-old-record.md").await;
    assert!(
        old_edit_form.contains("name=\"base_revision\" value=\"1\""),
        "a resolution failure must not bump the old record's revision: got: {old_edit_form}"
    );
}

#[tokio::test]
async fn superseding_is_refused_when_an_unrelated_record_already_fails_check_and_nothing_commits() {
    let router = supersede_fixture(&[(
        "0003-broken-record",
        "status: Superseded\nsuperseded_by: 9999\n",
        "Broken Record",
        "This record already fails check before any supersede runs.",
    )])
    .await;

    let response = post_form(router.clone(), "/supersede/adr/0001-old-record.md", "new=2").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(body.to_lowercase().contains("check failed"), "got: {body}");

    let old_body = get_ok_body(router.clone(), "/record/adr/0001-old-record.md").await;
    let old_aside = aside_pane(&old_body);
    assert!(
        !old_aside.contains("Superseded"),
        "a failing check must not commit: got: {old_body}"
    );

    let old_edit_form = get_ok_body(router, "/edit/adr/0001-old-record.md").await;
    assert!(
        old_edit_form.contains("name=\"base_revision\" value=\"1\""),
        "a failing check must not bump the old record's revision: got: {old_edit_form}"
    );
}

#[tokio::test]
async fn the_record_page_renders_a_supersede_form_in_db_mode_authoring() {
    let router = supersede_fixture(&[]).await;

    let body = get_ok_body(router, "/record/adr/0001-old-record.md").await;

    assert!(
        body.contains("<form action=\"/supersede/adr/0001-old-record.md\" method=\"post\""),
        "got: {body}"
    );
}

#[tokio::test]
async fn the_record_page_renders_no_supersede_form_in_file_mode() {
    let router = seeded_router().await;

    let body = get_ok_body(router, "/record/adr/0001-quokka-caching.md").await;

    assert!(!body.contains("class=\"supersede-form\""), "got: {body}");
}

#[tokio::test]
async fn supersede_route_is_unregistered_in_file_mode_returning_404() {
    let router = seeded_router().await;

    let response = post_form(router, "/supersede/adr/0001-quokka-caching.md", "new=0002").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

fn write_issue_record(
    docs: &Path,
    filename: &str,
    extra_frontmatter: &str,
    title: &str,
    body: &str,
) {
    let type_dir = docs.join("issues");
    fs::create_dir_all(&type_dir).expect("create issues dir");
    let contents = format!(
        "---\ntype: Issue\ntitle: {title}\ndescription: seeded for web http test\n{extra_frontmatter}---\n\n# {title}\n\n{body}\n"
    );
    fs::write(type_dir.join(filename), contents).expect("write seeded issue record");
}

/// The bundle-root `index.md` plus both type indices `check` needs reachable
/// (mirrors `db-store`'s own `seed_index_with_issues`, since the delete
/// tests need an `adr` fixture — an ineligible doc type — alongside the
/// eligible `issues` fixtures, unlike `seed_index_skeleton`, which only ever
/// links `adr`).
fn seed_delete_index(docs: &Path, adr_entries: &[&str], issue_entries: &[&str]) {
    fs::create_dir_all(docs.join("adr")).expect("create adr dir");
    fs::create_dir_all(docs.join("issues")).expect("create issues dir");
    fs::write(
        docs.join("index.md"),
        "# Index\n\n- [ADRs](adr/index.md)\n- [Issues](issues/index.md)\n",
    )
    .expect("write root index");
    write_delete_type_index(docs, "adr", adr_entries);
    write_delete_type_index(docs, "issues", issue_entries);
}

fn write_delete_type_index(docs: &Path, dir: &str, entries: &[&str]) {
    let rows: String = entries
        .iter()
        .map(|entry| format!("- [{entry}]({entry}.md)\n"))
        .collect();
    fs::write(
        docs.join(dir).join("index.md"),
        format!("# Index\n\n{rows}"),
    )
    .expect("write type index");
}

/// A db-mode-authoring router seeded with four records: an eligible,
/// relation-free issue (`issues/0001-eligible-issue.md`), an ineligible ADR
/// (`adr/0001-ineligible-adr.md`), and a target/source issue pair
/// (`issues/0002-target-issue.md`/`issues/0003-source-issue.md`) where the
/// source's `supersedes: 2` frontmatter gives the target an inbound
/// relation — the three shapes `db_store::DbDocStore::delete_checked`
/// distinguishes (ADR 0018, issue 0013 slice A/B).
async fn delete_fixture() -> Router {
    let docs = temp_dir("docs-authoring-delete");
    seed_delete_index(
        &docs,
        &["0001-ineligible-adr"],
        &[
            "0001-eligible-issue",
            "0002-target-issue",
            "0003-source-issue",
        ],
    );

    write_record(
        &docs,
        "adr",
        "0001-ineligible-adr.md",
        "Ineligible Decision",
        "An ADR record, whose doc type is not delete-eligible.",
    );
    write_issue_record(
        &docs,
        "0001-eligible-issue.md",
        "status: Open\n",
        "Eligible Issue",
        "An issue record with no inbound relations, eligible for delete.",
    );
    write_issue_record(
        &docs,
        "0002-target-issue.md",
        "status: Open\n",
        "Target Issue",
        "An issue record another record's relations row still points at.",
    );
    write_issue_record(
        &docs,
        "0003-source-issue.md",
        "status: Open\nsupersedes: 2\n",
        "Source Issue",
        "An issue record that supersedes (and thus points at) the target issue.",
    );

    let db_path = temp_dir("db-authoring-delete-parent").join("index.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let conn = db_store::connect(&db_url).await.expect("connect");
    db_store::migrate(&conn).await.expect("migrate");
    db_store::sync(&conn, &fs_store::FsStore::new(), &docs)
        .await
        .expect("sync seeded corpus");

    let authoring = web::AuthoringConfig {
        db_url,
        docs_root: docs,
    };
    web::build_router(conn, Some(authoring))
}

#[tokio::test]
async fn deleting_an_eligible_relation_free_issue_commits_and_the_record_page_shows_the_deleted_badge_with_no_delete_form(
) {
    let router = delete_fixture().await;

    let response = post_form(router.clone(), "/delete/issues/0001-eligible-issue.md", "").await;

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .expect("redirect carries a Location header")
        .to_str()
        .expect("location header is valid utf-8")
        .to_owned();
    assert_eq!(location, "/record/issues/0001-eligible-issue.md");

    let body = get_ok_body(router, &location).await;
    let aside = aside_pane(&body);
    assert!(
        aside.contains("class=\"status-badge status-deleted\""),
        "got: {body}"
    );
    assert!(aside.contains(">Deleted<"), "got: {body}");
    assert!(!body.contains("class=\"delete-form\""), "got: {body}");
}

#[tokio::test]
async fn deleting_an_ineligible_adr_is_rejected_and_leaves_deleted_at_unset() {
    let router = delete_fixture().await;

    let response = post_form(router.clone(), "/delete/adr/0001-ineligible-adr.md", "").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(body.contains("is not eligible for delete"), "got: {body}");

    let record_body = get_ok_body(router, "/record/adr/0001-ineligible-adr.md").await;
    let aside = aside_pane(&record_body);
    assert!(
        !aside.contains("status-deleted"),
        "a refused delete must leave deleted_at unset: got: {record_body}"
    );
}

#[tokio::test]
async fn deleting_an_issue_with_an_inbound_relation_is_rejected_and_leaves_deleted_at_unset() {
    let router = delete_fixture().await;

    let response = post_form(router.clone(), "/delete/issues/0002-target-issue.md", "").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_text(response).await;
    assert!(body.contains("form-error"), "got: {body}");
    assert!(
        body.to_lowercase()
            .contains("cannot delete a record another record still refers to"),
        "got: {body}"
    );

    let record_body = get_ok_body(router, "/record/issues/0002-target-issue.md").await;
    let aside = aside_pane(&record_body);
    assert!(
        !aside.contains("status-deleted"),
        "a refused delete must leave deleted_at unset: got: {record_body}"
    );
}

#[tokio::test]
async fn delete_route_is_unregistered_in_file_mode_returning_404() {
    let router = seeded_router().await;

    let response = post_form(router, "/delete/adr/0001-quokka-caching.md", "").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
