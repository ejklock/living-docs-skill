use crate::commands::new::unsupported_type_message;
use crate::commands::next::numeric_prefix as numbered_prefix;
use crate::doc_type::{self, Identity};
use crate::frontmatter;
use crate::line_ending::{ending_of, Ending, LineEndingStore, MIXED_MESSAGE};
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
                Identity::Numbered { .. } | Identity::Named { .. }
            )
        })
        .map(|spec| spec.token.to_string())
        .collect()
}

pub fn run(store: &dyn DocStore, docs_dir: &Path, doc_type: Option<String>) -> ExitCode {
    match write(store, docs_dir, doc_type) {
        Ok(written) => {
            written.iter().for_each(|p| println!("{}", p.display()));
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("living-docs index: {message}");
            ExitCode::from(2)
        }
    }
}

/// Regenerates `doc_type`'s index(es) and makes the bundle-root index reach
/// them, returning the written paths (a type with no directory yet, or an
/// index already current, is a silent no-op that lists nothing); [`run`]
/// renders these as plain text, the CLI front also as JSON. A mixed-ending
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
        docs_dir,
        &tokens,
        NoTypeIndex::WriteNothing,
    )?);
    Ok(written)
}

/// `index.md` is a reserved fs artifact outside every `DocStore` domain:
/// always read/written through `std::fs`, even in db-mode,
/// where only the records feeding its body come from `store`.
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
    fs::write(&index_path, content).map_err(|e| e.to_string())?;
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
    if fs::read_to_string(&index_path).is_ok_and(|on_disk| on_disk == content) {
        return Ok(None);
    }
    Ok(Some((index_path, content)))
}

/// Computes `doc_type`'s regenerated `index.md` path and content — reading
/// the on-disk preamble and the records through `store` — without writing
/// anything: the pure step [`regenerate`] and `db-store`'s `write_checked`
/// both build on, the latter controlling its own write/rollback timing.
pub fn compute(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
) -> Result<(PathBuf, String), String> {
    let type_dir = docs_dir.join(numbered_dir_for(doc_type)?);
    let index_path = type_dir.join("index.md");
    let existing = fs::read_to_string(&index_path).unwrap_or_default();
    let (existing, eol) = match ending_of(&existing) {
        Ending::Crlf => (existing.replace("\r\n", "\n"), "\r\n"),
        _ => (existing, "\n"),
    };
    let preamble = preamble_for(&existing, doc_type);
    let body = body_for(store, docs_dir, doc_type, &type_dir)?;
    Ok((index_path, format!("{preamble}{body}").replace('\n', eol)))
}

/// Renders `doc_type`'s listing body: a Named type delegates to
/// [`named::render_body`] (kind-ranked view rows), a Numbered one collects
/// `NNNN-*.md` records along its registry partition axis. Records are read in
/// their LF form, a mixed-ending one included.
fn body_for(
    store: &dyn DocStore,
    docs_dir: &Path,
    doc_type: &str,
    type_dir: &Path,
) -> Result<String, String> {
    let view = LineEndingStore::new(store, true);
    if is_named(doc_type) {
        return named::render_body(&view, docs_dir, type_dir);
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
            eprintln!("living-docs index: {}: {MIXED_MESSAGE}", path.display());
        }
    }
}

fn is_named(doc_type: &str) -> bool {
    let spec = doc_type::spec_for(doc_type);
    matches!(spec.map(|spec| spec.identity), Some(Identity::Named { .. }))
}

/// Resolves the numbered-series directory `index` regenerates for
/// `doc_type`: an unknown token gets [`unsupported_type_message`], but a
/// registered [`Identity::Singleton`] gets its own message — it IS
/// supported, it simply has no directory index.
fn numbered_dir_for(doc_type: &str) -> Result<&'static str, String> {
    let spec = doc_type::spec_for(doc_type).ok_or_else(|| unsupported_type_message(doc_type))?;
    match spec.identity {
        Identity::Numbered { dir } | Identity::Named { dir } => Ok(dir),
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
    let title = title_for_record(&contents, path, number);
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
/// parseable, otherwise its first `# ` H1 heading with a leading numbering
/// prefix stripped. A stderr warning names `path`
/// whenever the fallback fires; the title stays empty (still warned) only
/// when the H1 is also absent.
fn title_for_record(contents: &str, path: &Path, number: u32) -> String {
    if let Some(title) = frontmatter::read_scalar_from_str(contents, "title") {
        return title;
    }
    let fallback = first_heading(contents)
        .map(|heading| strip_heading_number_prefix(&heading, number))
        .unwrap_or_default();
    warn_missing_title(path, &fallback);
    fallback
}

fn first_heading(contents: &str) -> Option<String> {
    contents
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
}

/// Strips a leading `ADR NNNN — `, `NNNN. `, or `NNNN — ` numbering prefix
/// (`NNNN` being `number` zero-padded to four digits) from `heading`, in
/// that order, leaving it untouched when none matches.
fn strip_heading_number_prefix(heading: &str, number: u32) -> String {
    let padded = format!("{number:04}");
    [
        format!("ADR {padded} — "),
        format!("{padded}. "),
        format!("{padded} — "),
    ]
    .into_iter()
    .find_map(|prefix| heading.strip_prefix(prefix.as_str()).map(str::to_owned))
    .unwrap_or_else(|| heading.to_owned())
}

fn warn_missing_title(path: &Path, fallback: &str) {
    if fallback.is_empty() {
        eprintln!(
            "living-docs index: {} has no parseable 'title' frontmatter and no H1 heading; rendering an empty title",
            path.display()
        );
    } else {
        eprintln!(
            "living-docs index: {} has no parseable 'title' frontmatter; using its H1 heading {fallback:?}",
            path.display()
        );
    }
}

#[cfg(test)]
mod root_tests;
#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
