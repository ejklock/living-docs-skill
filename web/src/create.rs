//! Atlas's create path (ADR 0016): plans a record through
//! `living_docs_core::commands::new::plan`, substitutes the submitted
//! title, and commits through `db_store::DbDocStore::write_checked` so an
//! invalid record is rejected before it is ever visible.

use crate::views;
use crate::{create_form_response, relative_record_path, replace_title_line};
use crate::{AppState, CreateForm};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use living_docs_core::commands::new as new_cmd;
use std::path::PathBuf;

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
pub(crate) enum CreateError {
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
#[allow(clippy::too_many_lines)]
pub(crate) async fn create_handler(
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
pub(crate) fn create_record(
    db_url: &str,
    docs_root: &std::path::Path,
    doc_type: &str,
    title: &str,
) -> std::result::Result<PathBuf, CreateError> {
    let store = db_store::DbDocStore::new(db_url, docs_root.to_path_buf())
        .map_err(|err| CreateError::Plan(err.to_string()))?;
    let (target_path, filled) = new_cmd::plan(
        &store,
        docs_root,
        doc_type,
        title,
        &new_cmd::NewOptions::default(),
    )
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
pub(crate) fn fill_title(filled: &str, title: &str) -> String {
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
