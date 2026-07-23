//! Library surface shared between the `living-docs-web` binary and its
//! integration tests (ADR 0006, issue 0003 slices S3a-S3b; three-pane shell
//! ADR 0015, issue 0008 slices S2-S4; Atlas's authoring create route ADR
//! 0016, issue 0010 slice 3): the read-only axum router, its `GET /` search
//! handler, its `GET /record/{*path}` record handler (metadata panel fed by
//! `db_store::record_meta`, S3), its `GET /style.css` stylesheet route, the
//! Cmd+K palette's `GET /palette.js` static script plus its `GET /palette`
//! fragment endpoint (S4) reusing the same async `db_store` search path as
//! `GET /`, and — mounted only when the caller supplies an
//! [`AuthoringConfig`] — `GET /new`/`POST /new`, which commits through
//! `db_store::DbDocStore::write_checked` via [`tokio::task::spawn_blocking`],
//! and `POST /delete/{*path}`, which soft-deletes through
//! `db_store::DbDocStore::delete_checked` (ADR 0018, issue 0013 slice B).

pub mod views;

use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
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

/// The web front's authoring configuration (ADR 0016, issue 0010 slice 3):
/// the SQLite/FTS5 read-model's connection URL and the docs bundle root a
/// `db_store::DbDocStore` opens against inside `POST /new`'s handler.
/// Constructed by the binary only for `--backend db`; its absence is what
/// keeps `/new` unregistered in file-mode (see [`build_router`]).
#[derive(Clone)]
pub struct AuthoringConfig {
    pub db_url: String,
    pub docs_root: PathBuf,
}

/// The axum router's shared state: the read-model connection every route
/// uses, plus the optional [`AuthoringConfig`] that gates whether `/new` is
/// registered at all. A single `Clone` struct because axum's `State`
/// extractor requires one state type per router.
#[derive(Clone)]
struct AppState {
    conn: DatabaseConnection,
    authoring: Option<AuthoringConfig>,
}

/// Builds the router backed by `conn`. Always exposes the read-only
/// `GET /`, `GET /record/{*path}`, `GET /style.css`, `GET /palette.js`, and
/// `GET /palette` routes; additionally mounts `GET /new`/`POST /new`,
/// `GET /edit/{*path}`/`POST /edit/{*path}`, `POST /supersede/{*path}`, and
/// `POST /delete/{*path}` only when `authoring` is `Some` — in file-mode
/// (`authoring: None`) a request to any of them 404s the same way any
/// unknown path does, since the routes are never registered rather than
/// being refused at runtime (ADR 0016, issue 0010 slice 3; issue 0011;
/// issue 0012; ADR 0018, issue 0013 slice B).
pub fn build_router(conn: DatabaseConnection, authoring: Option<AuthoringConfig>) -> Router {
    let mut router = Router::new()
        .route("/", get(search_handler))
        .route("/record/{*path}", get(record_handler))
        .route("/style.css", get(style_handler))
        .route("/palette.js", get(palette_script_handler))
        .route("/palette", get(palette_handler));

    if authoring.is_some() {
        router = router
            .route("/new", get(new_form_handler).post(create_handler))
            .route("/edit/{*path}", get(edit_form_handler).post(edit_handler))
            .route("/supersede/{*path}", axum::routing::post(supersede_handler))
            .route("/delete/{*path}", axum::routing::post(delete_handler));
    }

    router.with_state(AppState { conn, authoring })
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
    State(AppState { conn, .. }): State<AppState>,
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
    State(AppState { conn, .. }): State<AppState>,
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

async fn record_handler(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    record_page_response(
        &state,
        &path,
        SupersedeSubmission::default(),
        DeleteSubmission::default(),
    )
    .await
}

/// A supersede confirm form submission's render state: the number the caller
/// last typed into the `new` field (empty for a fresh `GET`) and, when the
/// submission was just rejected, the error to show above the form —
/// [`record_page_response`]'s one varying input between [`record_handler`]'s
/// plain page and [`supersede_error_response`]'s re-render.
#[derive(Default)]
struct SupersedeSubmission<'a> {
    value: &'a str,
    error: Option<&'a str>,
}

/// A delete confirm form submission's render state: the error to show above
/// the form when the submission was just rejected — mirrors
/// [`SupersedeSubmission`] minus a `value` field, since the delete form
/// submits no data of its own.
#[derive(Default)]
struct DeleteSubmission<'a> {
    error: Option<&'a str>,
}

/// Renders `path`'s full record page — body, metadata panel, and (in
/// db-mode authoring) the Edit link, the supersede confirm form, and the
/// delete confirm form — the shared render [`record_handler`]'s plain `GET`,
/// [`supersede_error_response`]'s rejected-submission re-render, and
/// [`delete_handler`]'s rejected-submission re-render all need, so no call
/// site duplicates this lookup-and-render sequence. 404s the same way
/// [`not_found_response`] does when no record exists at `path`. The delete
/// form is never shown for a record that is already soft-deleted, or when
/// no record/meta was found at all (ADR 0018, issue 0013 slice B).
async fn record_page_response(
    state: &AppState,
    path: &str,
    supersede_submission: SupersedeSubmission<'_>,
    delete_submission: DeleteSubmission<'_>,
) -> Response {
    let nav = nav_entries_or_log(&state.conn).await;
    let Some(record) = record_or_log(&state.conn, path).await else {
        return not_found_response(&state.conn).await;
    };
    let meta = record_meta_or_log(&state.conn, path).await;
    let body_html = render_markdown(&record.body);
    let title = format!("{} — living-docs", record.title);
    let edit_href = state.authoring.is_some().then(|| format!("/edit/{path}"));
    let supersede_href = state
        .authoring
        .is_some()
        .then(|| format!("/supersede/{path}"));
    let supersede = supersede_href
        .as_deref()
        .map(|href| views::SupersedeFormState {
            href,
            value: supersede_submission.value,
            error: supersede_submission.error,
        });
    let delete_href = delete_href_for(state, path, meta.as_ref());
    let delete = delete_href.as_deref().map(|href| views::DeleteFormState {
        href,
        error: delete_submission.error,
    });
    let main = views::record_page(&body_html, edit_href.as_deref(), supersede, delete);
    let aside = meta.map(|meta| views::metadata_panel(&meta));
    let shell = views::shell(&title, &nav, Some(path), main, aside);
    (StatusCode::OK, shell).into_response()
}

/// `/delete/{path}`, when authoring is configured AND `meta` shows a record
/// that is not already soft-deleted — `None` when authoring is unconfigured,
/// when no meta was found at all, or when `deleted_at` is already set
/// (ADR 0018, issue 0013 slice B: a delete form is never shown for a record
/// that is already deleted).
fn delete_href_for(state: &AppState, path: &str, meta: Option<&RecordMeta>) -> Option<String> {
    state.authoring.as_ref()?;
    let is_deleted = meta.is_none_or(|meta| meta.deleted_at.is_some());
    (!is_deleted).then(|| format!("/delete/{path}"))
}

/// The center-pane content shown when a path resolves to no record —
/// [`record_handler`]'s own 404, and reused by the `/edit` handlers below so
/// every "no record at that path" response renders identically.
async fn not_found_response(conn: &DatabaseConnection) -> Response {
    let nav = nav_entries_or_log(conn).await;
    let shell = views::shell(
        "Not found — living-docs",
        &nav,
        None,
        views::not_found(),
        None,
    );
    (StatusCode::NOT_FOUND, shell).into_response()
}

/// `GET /new`'s handler, mounted only when [`AuthoringConfig`] is `Some`
/// (see [`build_router`]): renders an empty [`views::create_form`] inside
/// the same three-pane shell every other page uses.
async fn new_form_handler(State(AppState { conn, .. }): State<AppState>) -> Markup {
    let nav = nav_entries_or_log(&conn).await;
    let main = views::create_form(None, None, None);
    views::shell("New record — living-docs", &nav, None, main, None)
}

/// `POST /new`'s submitted fields: the doc-type token
/// (`adr`/`bdr`/`prd`/`issue`) and the record's title.
#[derive(Debug, Deserialize)]
pub struct CreateForm {
    doc_type: String,
    title: String,
}

/// Every way `POST /new`'s handler can fail to commit a new record:
/// [`living_docs_core::commands::new::plan`]'s own `String` error (an
/// unsupported doc type, or a path the store already serves), or
/// [`db_store::DbDocStore::write_checked`]'s own
/// [`db_store::WriteCheckedError`] (most commonly a failing `check`).
/// Opening the store itself (`DbDocStore::new`) folds into
/// [`CreateError::Plan`] too — from the form submitter's point of view both
/// are "this submission could not be planned", surfaced identically by
/// [`views::create_form`]'s error slot.
#[derive(Debug)]
enum CreateError {
    Plan(String),
    Write(db_store::WriteCheckedError),
}

impl std::fmt::Display for CreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateError::Plan(message) => write!(f, "{message}"),
            CreateError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// `POST /new`'s handler, mounted only when [`AuthoringConfig`] is `Some`
/// (see [`build_router`]) — the `.expect` below is therefore always
/// satisfied. Every `DbDocStore`/`write_checked` call happens inside
/// [`tokio::task::spawn_blocking`]: `DbDocStore` bridges its own
/// synchronous SeaORM runtime and must never be driven from this handler's
/// own async task. On success, redirects (`303 See Other`) to the new
/// record's page; on a [`CreateError`], re-renders [`views::create_form`]
/// with the submitted fields preserved and the error's `Display` shown; a
/// panicked blocking task becomes a `500`.
async fn create_handler(
    State(state): State<AppState>,
    axum::Form(input): axum::Form<CreateForm>,
) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("create_handler is only mounted when authoring is configured");
    let CreateForm { doc_type, title } = input;
    let docs_root = authoring.docs_root.clone();
    let plan_doc_type = doc_type.clone();
    let plan_title = title.clone();

    let outcome = tokio::task::spawn_blocking(move || {
        create_record(
            &authoring.db_url,
            &authoring.docs_root,
            &plan_doc_type,
            &plan_title,
        )
    })
    .await;

    match outcome {
        Ok(Ok(target_path)) => {
            let relative = relative_record_path(&docs_root, &target_path);
            Redirect::to(&views::record_href(&relative)).into_response()
        }
        Ok(Err(err)) => {
            create_form_response(&state.conn, &doc_type, &title, &err.to_string()).await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error creating record",
        )
            .into_response(),
    }
}

/// Plans and commits one new record: [`living_docs_core::commands::new::plan`]
/// computes the target path and the template's filled content, [`fill_title`]
/// substitutes the submitted `title` for the template's own title
/// placeholder (never done by `plan` itself — it treats a record's title as
/// judgment for the authoring model, but Atlas's minimal create form has no
/// separate title-editing step), and
/// [`db_store::DbDocStore::write_checked`] commits only if the resulting
/// project state still passes `check`.
fn create_record(
    db_url: &str,
    docs_root: &std::path::Path,
    doc_type: &str,
    title: &str,
) -> std::result::Result<PathBuf, CreateError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| CreateError::Plan(err.to_string()))?;
    let (target_path, filled) =
        living_docs_core::commands::new::plan(&store, docs_root, doc_type, title)
            .map_err(CreateError::Plan)?;
    let filled = fill_title(&filled, title);
    store
        .write_checked(&target_path, &filled)
        .map(|_revision| target_path)
        .map_err(CreateError::Write)
}

/// Substitutes `title` for the frontmatter `title:` line's placeholder
/// value, the one field `living_docs_core::commands::new::fill_frontmatter`
/// deliberately leaves untouched — mirrors that function's own bounded,
/// guidance-comment-preserving, line-targeted replace, scoped to the
/// frontmatter block only (before its closing `---`), so nothing outside it
/// is ever touched. `title` is YAML-double-quote-escaped rather than
/// substituted raw: an unescaped colon or quote in a free-text browser
/// field would otherwise produce malformed frontmatter.
fn fill_title(filled: &str, title: &str) -> String {
    let lines: Vec<&str> = filled.lines().collect();
    let Some(close) = lines
        .iter()
        .skip(1)
        .position(|&line| line == "---")
        .map(|index| index + 1)
    else {
        return filled.to_owned();
    };

    let updated: Vec<String> = lines
        .iter()
        .enumerate()
        .map(|(index, &line)| {
            if index == 0 || index >= close {
                line.to_owned()
            } else {
                replace_title_line(line, title).unwrap_or_else(|| line.to_owned())
            }
        })
        .collect();
    updated.join("\n") + "\n"
}

fn replace_title_line(line: &str, title: &str) -> Option<String> {
    let rest = line.strip_prefix("title:")?;
    let quoted = yaml_double_quoted(title);
    match rest.find('#') {
        Some(hash) => Some(format!("title: {quoted} {}", &rest[hash..])),
        None => Some(format!("title: {quoted}")),
    }
}

/// A YAML double-quoted scalar for `value`: backslashes and double quotes
/// are escaped and newlines collapse to a space, so a free-text title
/// (unlike `plan`'s own type/status/timestamp fills, which never carry
/// user-controlled content) can never produce malformed frontmatter.
fn yaml_double_quoted(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push(' '),
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

/// `target_path` (docs-root-joined) rendered relative to `docs_root` —
/// mirrors `db_store::DbDocStore`'s own private `relative_path` so a
/// freshly created record's redirect target matches exactly what
/// `GET /record/{*path}` will read it back at.
fn relative_record_path(docs_root: &std::path::Path, target_path: &std::path::Path) -> String {
    target_path
        .strip_prefix(docs_root)
        .unwrap_or(target_path)
        .to_string_lossy()
        .into_owned()
}

async fn create_form_response(
    conn: &DatabaseConnection,
    doc_type: &str,
    title: &str,
    error: &str,
) -> Response {
    let nav = nav_entries_or_log(conn).await;
    let main = views::create_form(Some(doc_type), Some(title), Some(error));
    let shell = views::shell("New record — living-docs", &nav, None, main, None);
    (StatusCode::OK, shell).into_response()
}

/// `GET /edit/{*path}`'s handler, mounted only when [`AuthoringConfig`] is
/// `Some` (see [`build_router`]): loads the record's current content and
/// `revision` via [`db_store::DbDocStore::read_with_revision`] inside
/// [`tokio::task::spawn_blocking`] and pre-fills [`views::edit_form`] with
/// them. 404s the same way [`record_handler`] does when no record exists at
/// `path`.
async fn edit_form_handler(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("edit_form_handler is only mounted when authoring is configured");

    match spawn_read_with_revision(&authoring, &path).await {
        Ok(Some((content, revision))) => {
            edit_form_response(&state.conn, &path, &content, revision, None).await
        }
        Ok(None) => not_found_response(&state.conn).await,
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error loading record",
        )
            .into_response(),
    }
}

/// `POST /edit/{*path}`'s submitted fields: the edited markdown source and
/// the `revision` the editor last read, carried as a hidden field so
/// [`db_store::DbDocStore::update_checked`] can enforce ADR 0016's
/// optimistic-concurrency precondition.
#[derive(Debug, Deserialize)]
pub struct EditForm {
    content: String,
    base_revision: i64,
}

/// Every way `POST /edit/{*path}`'s handler can fail to commit an edit:
/// opening the store itself failed (folds together with a `Plan`-shaped
/// failure the way [`CreateError::Plan`] does), or
/// [`db_store::DbDocStore::update_checked`]'s own
/// [`db_store::WriteCheckedError`] — most commonly a stale `base_revision`
/// or a failing `check`.
#[derive(Debug)]
enum EditError {
    Open(String),
    Write(db_store::WriteCheckedError),
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditError::Open(message) => write!(f, "{message}"),
            EditError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// Normalizes `value`'s line endings to bare `\n`: real browsers submit
/// `<textarea>` content with CRLF (`\r\n`) per the HTML forms spec
/// regardless of what the DOM value holds, and living-docs-core's
/// frontmatter parser splits on bare `\n`, so a stray `\r` left on the
/// `type:`/closing-fence line breaks the match. Collapses `\r\n` first, then
/// any lone remaining `\r`, so both CRLF and old-style CR-only input land on
/// the same LF-only content [`db_store::DbDocStore::update_checked`] checks.
fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

/// `POST /edit/{*path}`'s handler, mounted only when [`AuthoringConfig`] is
/// `Some` (see [`build_router`]) — the `.expect` below is therefore always
/// satisfied. Runs [`db_store::DbDocStore::update_checked`] inside
/// [`tokio::task::spawn_blocking`], exactly like [`create_handler`]. On
/// success, redirects (`303 See Other`) to the record's page; on a stale
/// `base_revision`, re-renders [`views::edit_form`] with the CURRENT server
/// content and revision plus the conflict message — never the user's
/// rejected submission, since ADR 0016 rejects any merge/diff resolution;
/// on any other [`EditError`], re-renders with the user's OWN submitted
/// content and `base_revision` preserved so they can fix and resubmit. A
/// panicked blocking task becomes a `500`.
async fn edit_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    axum::Form(input): axum::Form<EditForm>,
) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("edit_handler is only mounted when authoring is configured");
    let EditForm {
        content,
        base_revision,
    } = input;
    let content = normalize_line_endings(&content);

    let outcome = spawn_update_checked(&authoring, &path, &content, base_revision).await;

    match outcome {
        Ok(Ok(_revision)) => Redirect::to(&views::record_href(&path)).into_response(),
        Ok(Err(err)) => {
            edit_error_response(&state, &authoring, &path, &content, base_revision, err).await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error editing record",
        )
            .into_response(),
    }
}

/// Runs [`db_store::DbDocStore::read_with_revision`] inside
/// [`tokio::task::spawn_blocking`], folding "no record at `path`" into
/// `Ok(None)` rather than an [`EditError`] — the read side of an edit has
/// no error to preserve a failed submission against, unlike the write side.
async fn spawn_read_with_revision(
    authoring: &AuthoringConfig,
    path: &str,
) -> std::result::Result<Option<(String, i64)>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || read_record_with_revision(&db_url, &docs_root, &path)).await
}

fn read_record_with_revision(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
) -> Option<(String, i64)> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf()).ok()?;
    store.read_with_revision(std::path::Path::new(path)).ok()
}

/// Runs [`update_record`] inside [`tokio::task::spawn_blocking`], mirroring
/// [`create_handler`]'s own bridge from this async handler onto
/// `DbDocStore`'s synchronous SeaORM runtime.
async fn spawn_update_checked(
    authoring: &AuthoringConfig,
    path: &str,
    content: &str,
    base_revision: i64,
) -> std::result::Result<std::result::Result<i64, EditError>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    let content = content.to_owned();
    tokio::task::spawn_blocking(move || {
        update_record(&db_url, &docs_root, &path, &content, base_revision)
    })
    .await
}

fn update_record(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
    content: &str,
    base_revision: i64,
) -> std::result::Result<i64, EditError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| EditError::Open(err.to_string()))?;
    store
        .update_checked(std::path::Path::new(path), content, Some(base_revision))
        .map_err(EditError::Write)
}

/// Routes an [`EditError`] to the right re-render: a stale `base_revision`
/// reloads the CURRENT server content (never the rejected submission, ADR
/// 0016); any other error preserves the user's own submitted `content`/
/// `base_revision` so they can fix and resubmit.
async fn edit_error_response(
    state: &AppState,
    authoring: &AuthoringConfig,
    path: &str,
    content: &str,
    base_revision: i64,
    err: EditError,
) -> Response {
    let message = err.to_string();
    if matches!(
        err,
        EditError::Write(db_store::WriteCheckedError::StaleRevision { .. })
    ) {
        return stale_edit_response(state, authoring, path, &message).await;
    }
    edit_form_response(&state.conn, path, content, base_revision, Some(&message)).await
}

/// Re-fetches `path`'s CURRENT server content and revision and re-renders
/// [`views::edit_form`] with them plus `message` — the stale-revision
/// branch of [`edit_error_response`], kept separate to hold that function to
/// a flat sequence of steps. Falls back to [`not_found_response`] in the
/// (pathological) case the record was deleted between the rejected edit and
/// this re-read.
async fn stale_edit_response(
    state: &AppState,
    authoring: &AuthoringConfig,
    path: &str,
    message: &str,
) -> Response {
    match spawn_read_with_revision(authoring, path)
        .await
        .ok()
        .flatten()
    {
        Some((content, revision)) => {
            edit_form_response(&state.conn, path, &content, revision, Some(message)).await
        }
        None => not_found_response(&state.conn).await,
    }
}

async fn edit_form_response(
    conn: &DatabaseConnection,
    path: &str,
    content: &str,
    base_revision: i64,
    error: Option<&str>,
) -> Response {
    let nav = nav_entries_or_log(conn).await;
    let main = views::edit_form(path, content, base_revision, error);
    let shell = views::shell("Edit record — living-docs", &nav, Some(path), main, None);
    (StatusCode::OK, shell).into_response()
}

/// `POST /supersede/{*path}`'s submitted field: the superseding record's
/// bare number (issue 0012) — matching the CLI's own `living-docs
/// supersede` argument shape, never a path.
#[derive(Debug, Deserialize)]
pub struct SupersedeForm {
    new: String,
}

/// Every way `POST /supersede/{*path}`'s handler can fail to commit: reading
/// the current record or deriving its number failed (folds together with a
/// `Plan`-shaped failure the way [`CreateError::Plan`]/[`EditError::Open`]
/// do), or [`db_store::DbDocStore::supersede_checked`]'s own
/// [`db_store::SupersedeCheckedError`] — most commonly an unresolvable
/// target number or a failing `check`.
#[derive(Debug)]
enum SupersedeError {
    Open(String),
    Write(db_store::SupersedeCheckedError),
}

impl std::fmt::Display for SupersedeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SupersedeError::Open(message) => write!(f, "{message}"),
            SupersedeError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// `POST /supersede/{*path}`'s handler, mounted only when
/// [`AuthoringConfig`] is `Some` (see [`build_router`]) — the `.expect`
/// below is therefore always satisfied. Runs
/// [`db_store::DbDocStore::supersede_checked`] inside
/// [`tokio::task::spawn_blocking`], exactly like [`create_handler`]/
/// [`edit_handler`]. On success, redirects (`303 See Other`) to `path`'s own
/// record page (issue 0012: the old record stays the page the browser lands
/// on, mirroring the CLI's own two-argument shape where `old` is the
/// caller's frame of reference); on a [`SupersedeError`], re-renders `path`'s
/// record page with the error visible and the submitted `new` value
/// preserved in the form. A panicked blocking task becomes a `500`.
async fn supersede_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
    axum::Form(input): axum::Form<SupersedeForm>,
) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("supersede_handler is only mounted when authoring is configured");
    let new = input.new.trim().to_owned();

    let outcome = spawn_supersede_checked(&authoring, &path, &new).await;

    match outcome {
        Ok(Ok(())) => Redirect::to(&views::record_href(&path)).into_response(),
        Ok(Err(err)) => {
            let message = err.to_string();
            record_page_response(
                &state,
                &path,
                SupersedeSubmission {
                    value: &new,
                    error: Some(&message),
                },
                DeleteSubmission::default(),
            )
            .await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error superseding record",
        )
            .into_response(),
    }
}

/// Runs [`supersede_record`] inside [`tokio::task::spawn_blocking`],
/// mirroring [`spawn_update_checked`]'s own bridge from this async handler
/// onto `DbDocStore`'s synchronous SeaORM runtime.
async fn spawn_supersede_checked(
    authoring: &AuthoringConfig,
    path: &str,
    new: &str,
) -> std::result::Result<std::result::Result<(), SupersedeError>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    let new = new.to_owned();
    tokio::task::spawn_blocking(move || supersede_record(&db_url, &docs_root, &path, &new)).await
}

/// Reads `path`'s current content to derive its own record number (the
/// `old` argument [`db_store::DbDocStore::supersede_checked`] needs — the
/// URL carries the record's path, not its bare number), then delegates to
/// `supersede_checked` with `new` exactly as submitted.
fn supersede_record(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
    new: &str,
) -> std::result::Result<(), SupersedeError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| SupersedeError::Open(err.to_string()))?;
    let (content, _revision) = store
        .read_with_revision(std::path::Path::new(path))
        .map_err(|err| SupersedeError::Open(err.to_string()))?;
    let old_number = record_number(path, &content).ok_or_else(|| {
        SupersedeError::Open(format!("{path}: record carries no number identity"))
    })?;
    store
        .supersede_checked(&old_number, new)
        .map_err(SupersedeError::Write)
}

/// `path`'s record's own zero-padded `NNNN` number, derived from its
/// filename exactly as [`db_store::record::extract_record`] derives every
/// record's identity — `None` for a doc type with no numbered identity
/// (issue 0012 scopes supersede to numbered doc types only, matching the
/// CLI's own precondition).
fn record_number(path: &str, content: &str) -> Option<String> {
    db_store::record::extract_record(std::path::Path::new(path), content)
        .number
        .map(|number| format!("{number:04}"))
}

/// Every way `POST /delete/{*path}`'s handler can fail to commit: opening
/// the store itself failed (folds together with an `Open`-shaped failure
/// the way [`SupersedeError::Open`] does), or
/// [`db_store::DbDocStore::delete_checked`]'s own
/// [`db_store::DeleteCheckedError`] — most commonly an ineligible doc type
/// or a record another record still refers to.
#[derive(Debug)]
enum DeleteError {
    Open(String),
    Write(db_store::DeleteCheckedError),
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteError::Open(message) => write!(f, "{message}"),
            DeleteError::Write(err) => write!(f, "{err}"),
        }
    }
}

/// `POST /delete/{*path}`'s handler, mounted only when [`AuthoringConfig`]
/// is `Some` (see [`build_router`]) — the `.expect` below is therefore
/// always satisfied. The delete form submits no fields, so unlike the other
/// authoring handlers this one takes no `axum::Form` extractor. Runs
/// [`db_store::DbDocStore::delete_checked`] inside
/// [`tokio::task::spawn_blocking`], exactly like [`supersede_handler`]. On
/// success, redirects (`303 See Other`) to `path`'s own record page — ADR
/// 0018 keeps a soft-deleted record viewable, so the page the browser lands
/// on is unchanged from every other write handler's own success path; on a
/// [`DeleteError`], re-renders `path`'s record page with the error visible.
/// A panicked blocking task becomes a `500`.
async fn delete_handler(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    let authoring = state
        .authoring
        .clone()
        .expect("delete_handler is only mounted when authoring is configured");

    let outcome = spawn_delete_checked(&authoring, &path).await;

    match outcome {
        Ok(Ok(())) => Redirect::to(&views::record_href(&path)).into_response(),
        Ok(Err(err)) => {
            let message = err.to_string();
            record_page_response(
                &state,
                &path,
                SupersedeSubmission::default(),
                DeleteSubmission {
                    error: Some(&message),
                },
            )
            .await
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal error deleting record",
        )
            .into_response(),
    }
}

/// Runs [`delete_record`] inside [`tokio::task::spawn_blocking`], mirroring
/// [`spawn_supersede_checked`]'s own bridge from this async handler onto
/// `DbDocStore`'s synchronous SeaORM runtime.
async fn spawn_delete_checked(
    authoring: &AuthoringConfig,
    path: &str,
) -> std::result::Result<std::result::Result<(), DeleteError>, tokio::task::JoinError> {
    let db_url = authoring.db_url.clone();
    let docs_root = authoring.docs_root.clone();
    let path = path.to_owned();
    tokio::task::spawn_blocking(move || delete_record(&db_url, &docs_root, &path)).await
}

fn delete_record(
    db_url: &str,
    docs_root: &std::path::Path,
    path: &str,
) -> std::result::Result<(), DeleteError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| DeleteError::Open(err.to_string()))?;
    store
        .delete_checked(std::path::Path::new(path))
        .map_err(DeleteError::Write)
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEMPLATE: &str = "---\ntype: ADR\ntitle: <Short decision title>\ndescription: <One sentence.>\nstatus: Proposed            # Proposed | Accepted\ntimestamp: <ISO 8601 datetime>\n---\n\n# NNNN. <Short decision title>\n\nBody.\n";

    #[test]
    fn fill_title_replaces_only_the_frontmatter_title_line() {
        let filled = fill_title(TEMPLATE, "New Feature");

        assert!(filled.contains("title: \"New Feature\"\n"));
        assert!(filled.contains("# NNNN. <Short decision title>"));
        assert!(filled.contains("description: <One sentence.>"));
    }

    #[test]
    fn fill_title_preserves_a_trailing_guidance_comment() {
        let template = "---\ntitle: <placeholder>          # Fill this in\n---\nBody.\n";

        let filled = fill_title(template, "My Title");

        assert!(filled.contains("title: \"My Title\""));
        assert!(filled.contains("# Fill this in"));
    }

    #[test]
    fn fill_title_leaves_content_unchanged_without_a_closing_frontmatter_fence() {
        let no_fence = "title: <placeholder>\nBody with no frontmatter fence.\n";

        assert_eq!(fill_title(no_fence, "My Title"), no_fence);
    }

    #[test]
    fn fill_title_escapes_a_colon_and_a_double_quote_so_the_frontmatter_stays_valid_yaml() {
        let filled = fill_title(TEMPLATE, "Weird: \"Title\"");

        assert!(filled.contains("title: \"Weird: \\\"Title\\\"\"\n"));
    }

    #[test]
    fn relative_record_path_strips_the_docs_root_prefix() {
        let docs_root = std::path::Path::new("/bundle/docs");
        let target = docs_root.join("adr").join("0002-new-feature.md");

        assert_eq!(
            relative_record_path(docs_root, &target),
            "adr/0002-new-feature.md"
        );
    }

    #[test]
    fn normalize_line_endings_collapses_crlf_to_lf() {
        let crlf = "---\r\ntype: ADR\r\ntitle: \"X\"\r\n---\r\n\r\nBody.\r\n";

        let normalized = normalize_line_endings(crlf);

        assert_eq!(normalized, "---\ntype: ADR\ntitle: \"X\"\n---\n\nBody.\n");
        assert!(!normalized.contains('\r'));
    }

    #[test]
    fn normalize_line_endings_collapses_lone_cr_and_leaves_lf_only_content_unchanged() {
        assert_eq!(normalize_line_endings("a\rb\nc"), "a\nb\nc");
        assert_eq!(
            normalize_line_endings("already\nlf\nonly\n"),
            "already\nlf\nonly\n"
        );
    }

    #[test]
    fn create_error_display_surfaces_the_plan_message_and_the_write_error() {
        let plan_error = CreateError::Plan("adr/0002-taken.md already exists".to_owned());
        assert_eq!(plan_error.to_string(), "adr/0002-taken.md already exists");

        let write_error = CreateError::Write(db_store::WriteCheckedError::AlreadyExists(
            "adr/0002-taken.md".to_owned(),
        ));
        assert_eq!(write_error.to_string(), "adr/0002-taken.md already exists");
    }
}
