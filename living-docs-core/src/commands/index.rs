use crate::commands::new::unsupported_type_message;
use crate::commands::next::numeric_prefix as numbered_prefix;
use crate::doc_type::{self, Identity};
use crate::frontmatter;
use crate::line_ending::{ending_of, Ending, LineEndingStore, MIXED_MESSAGE};
use crate::record::{fallback_title, first_heading};
use crate::store::{is_refusal, DocStore};
use std::path::{Path, PathBuf};

mod named;
mod preamble;
mod refresh;
mod root;
mod rows;

use preamble::preamble_for;
#[cfg(test)]
use preamble::{fallback_preamble, find_boundary_offset, is_boundary_line};
pub use refresh::{refresh_for_records, regenerate_for_record, IndexFailure, Indexed};
pub(crate) use root::{ensure_root_index, NoTypeIndex};

/// Every registry token with a directory to index (Numbered and Named
/// identities), in [`doc_type::DOC_TYPES`] order — what a
/// bare `index` regenerates. A [`Identity::Singleton`] has no directory, so
/// the sweep excludes it.
fn all_type_tokens() -> Vec<String> {
    doc_type::DOC_TYPES
        .iter()
        .filter(|spec| {
            matches!(
                spec.identity,
                Identity::Numbered { .. } | Identity::Named { .. } | Identity::Nested { .. }
            )
        })
        .map(|spec| spec.token.to_string())
        .collect()
}

/// Regenerates `doc_type`'s index(es) and makes the bundle-root index reach
/// them, returning the written paths (a type with no directory yet, or an
/// index already current, is a silent no-op that lists nothing); the
/// CLI front renders these as plain text or JSON. A mixed-ending
/// record keeps its row and draws a stderr warning, from this verb only.
pub fn write(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    let types: Vec<String> = match doc_type {
        Some(t) => vec![t],
        None => all_type_tokens(),
    };
    types
        .iter()
        .for_each(|t| warn_mixed_rows(store, docs_dir, t));
    let mut written = types
        .iter()
        .filter_map(|doc_type| regenerate(store, docs_dir, doc_type).transpose())
        .collect::<Result<Vec<PathBuf>, String>>()?;
    let tokens: Vec<&str> = types.iter().map(String::as_str).collect();
    written.extend(ensure_root_index(
        store,
        docs_dir,
        &tokens,
        NoTypeIndex::WriteNothing,
    )?);
    Ok(written)
}

/// Reads a reserved `index.md` through `store`, so a store rooted at the
/// bundle judges it like any record: `Some(text)` when read, `None` when it
/// cannot be read for any reason but a refusal (a missing index is a fresh
/// one), and an error naming the path when the store refuses it, so a caller
/// never regenerates over a file that lies outside the bundle.
pub(crate) fn read_reserved(store: &dyn DocStore, path: &Path) -> Result<Option<String>, String> {
    match store.read(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if is_refusal(&error) => Err(error.to_string()),
        Err(_) => Ok(None),
    }
}

/// `index.md` is read and written through `store`, like the records feeding
/// its body, so a store rooted at the bundle refuses one that resolves outside
/// it.
///
/// A type with no directory yet is a silent no-op — `new` owns
/// directory creation, never `index`; regenerating one would materialize an
/// empty `index.md` for every unused registry token, breaking invariant 3.
fn regenerate(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> Result<Option<PathBuf>, String> {
    let Some((index_path, content)) = pending(store, docs_dir, doc_type)? else {
        return Ok(None);
    };
    store
        .write(&index_path, &content)
        .map_err(|e| e.to_string())?;
    Ok(Some(index_path))
}

/// The regeneration [`regenerate`] would write for `doc_type` — its path and
/// content — or `None` when the type has no directory yet or the index on
/// disk already equals it. Nothing is written, so a read-only caller (the
/// index-drift check) judges an index by the very computation `index` runs.
pub(crate) fn pending(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> Result<Option<(PathBuf, String)>, String> {
    let (index_path, content) = compute(store, docs_dir, doc_type)?;
    let type_dir = index_path.parent().unwrap_or(docs_dir);
    if !type_dir.is_dir() {
        return Ok(None);
    }
    if read_reserved(store, &index_path)?.is_some_and(|on_disk| on_disk == content) {
        return Ok(None);
    }
    Ok(Some((index_path, content)))
}

/// Computes `doc_type`'s regenerated `index.md` path and content without
/// writing anything. The on-disk preamble and the records are both read
/// through `store`, so an index the store refuses is an error, not an empty
/// preamble. [`regenerate`] writes the result; a caller that controls its own
/// write timing uses this step alone.
pub fn compute(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> Result<(PathBuf, String), String> {
    let type_dir = docs_dir.join(numbered_dir_for(doc_type)?);
    let index_path = type_dir.join("index.md");
    let existing = read_reserved(store, &index_path)?.unwrap_or_default();
    let (existing, eol) = match ending_of(&existing) {
        Ending::Crlf => (existing.replace("\r\n", "\n"), "\r\n"),
        _ => (existing, "\n"),
    };
    let preamble = preamble_for(&existing, doc_type);
    let body = body_for(store, docs_dir, doc_type, &type_dir)?;
    Ok((index_path, format!("{preamble}{body}").replace('\n', eol)))
}

/// Renders `doc_type`'s listing body: a Named type delegates to
/// [`named::render_body`] (kind-ranked view rows), a Nested one to the same
/// renderer unranked (rows grouped by folder, at any depth), a Numbered one collects
/// `NNNN-*.md` records along its registry partition axis. Records are read in
/// their LF form, a mixed-ending one included.
fn body_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    type_dir: &Path,
) -> Result<String, String> {
    let view = LineEndingStore::new(store, true);
    match doc_type::spec_for(doc_type).map(|spec| spec.identity) {
        Some(Identity::Named { .. }) => return named::render_body(&view, docs_dir, type_dir, true),
        Some(Identity::Nested { .. }) => {
            return named::render_body(&view, docs_dir, type_dir, false)
        }
        _ => {}
    }
    let records = collect_records(&view, docs_dir, type_dir)?;
    Ok(rows::render_body(doc_type, &records))
}

/// Warns on stderr, with the repair, about each row record of `doc_type`'s
/// directory that mixes CRLF and bare LF breaks.
fn warn_mixed_rows(store: &dyn DocStore, docs_dir: &Path, doc_type: &str) {
    let Ok(dir) = numbered_dir_for(doc_type) else {
        return;
    };
    let type_dir = docs_dir.join(dir);
    let is_row = |name: &str| match is_named(doc_type) {
        true => name.ends_with(".md") && name != "index.md",
        false => numbered_prefix(name).is_some(),
    };
    let is_mixed = |p: &Path| ending_of(&store.read(p).unwrap_or_default()) == Ending::Mixed;
    for path in store.list(docs_dir).unwrap_or_default() {
        let name = path.file_name().and_then(|name| name.to_str());
        if path.parent() == Some(&type_dir) && name.is_some_and(is_row) && is_mixed(&path) {
            eprintln!(
                "living-docs manage index: {}: {MIXED_MESSAGE}",
                path.display()
            );
        }
    }
}

fn is_named(doc_type: &str) -> bool {
    let spec = doc_type::spec_for(doc_type);
    matches!(
        spec.map(|spec| spec.identity),
        Some(Identity::Named { .. } | Identity::Nested { .. })
    )
}

/// Resolves the numbered-series directory `index` regenerates for
/// `doc_type`: an unknown token gets [`unsupported_type_message`], but a
/// registered [`Identity::Singleton`] gets its own message — it IS
/// supported, it simply has no directory index.
fn numbered_dir_for(doc_type: &str) -> Result<&'static str, String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    match spec.identity {
        Identity::Numbered { dir } | Identity::Named { dir } | Identity::Nested { dir } => Ok(dir),
        Identity::Singleton { file } => Err(format!(
            "'{doc_type}' has no directory index — it writes a single {file} at the bundle root, not a numbered series"
        )),
    }
}

struct Record {
    number: u32,
    title: String,
    status: String,
    filename: String,
    superseded_by: Option<String>,
}

/// Every `NNNN-*.md` record directly under `type_dir`, sorted ascending by
/// `NNNN`, read through `store` (backend-faithful). `title`/`status` come
/// from each record's frontmatter; `NNNN` from the filename.
fn collect_records(
    store: &dyn DocStore,
    docs_dir: &Path,
    type_dir: &Path,
) -> Result<Vec<Record>, String> {
    let paths = store.list(docs_dir).map_err(|e| e.to_string())?;

    let mut records: Vec<Record> = paths
        .iter()
        .filter(|path| path.parent() == Some(type_dir))
        .filter_map(|path| record_from_path(store, path))
        .collect();

    records.sort_by_key(|record| record.number);
    Ok(records)
}

fn record_from_path(store: &dyn DocStore, path: &Path) -> Option<Record> {
    let filename = path.file_name()?.to_str()?.to_string();
    let number = numbered_prefix(&filename)?;
    let contents = store.read(path).ok()?;
    let title = title_for_record(&contents, path);
    let status = frontmatter::read_scalar_from_str(&contents, "status").unwrap_or_default();
    let superseded_by = frontmatter::read_scalar_from_str(&contents, "superseded_by");
    Some(Record {
        number,
        title,
        status,
        filename,
        superseded_by,
    })
}

/// The record's rendered title: its frontmatter `title:` when present and
/// parseable, otherwise the title [`fallback_title`] derives, the same one
/// `read` shows. A stderr warning names `path` and that title whenever the
/// fallback fires.
fn title_for_record(contents: &str, path: &Path) -> String {
    if let Some(title) = frontmatter::read_scalar_from_str(contents, "title") {
        return title;
    }
    let fallback = fallback_title(path, contents);
    eprintln!(
        "living-docs manage index: {} has no parseable 'title' frontmatter; using {fallback:?}",
        path.display()
    );
    fallback
}

#[cfg(test)]
mod root_tests;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
