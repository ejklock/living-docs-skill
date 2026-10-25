//! Retired-record callout: the imperative banner a Superseded or Deprecated
//! record's body must open with (above its H1), so an agent that lands on
//! the record's raw markdown sees the stop rule before anything else.
//! Detection, reconciliation, and the store-level write live here as the
//! single home every CLI-writing call site (`supersede`, `set`, `fmt`)
//! shares, so the exact wording never drifts between them.

use crate::commands::supersede::guard::{refusal_message, unwritable_key};
use crate::record::{extract_record, to_canonical_markdown};
use crate::store::DocStore;
use std::path::Path;

const SUPERSEDED_MARKER: &str = "> **SUPERSEDED";
const DEPRECATED_MARKER: &str = "> **DEPRECATED";

/// The exact callout line a record with `status` must open with, `None`
/// for any status other than Superseded or Deprecated (case-insensitive).
/// For Superseded, `successor` is the resolved link target — a sibling
/// filename or, when none resolves, the bare zero-padded `superseded_by`
/// number — and its leading digit run supplies the `[NNNN]` link text.
pub fn expected(status: Option<&str>, successor: Option<&str>) -> Option<String> {
    let status = status?.to_lowercase();
    match status.as_str() {
        "superseded" => Some(superseded_callout(successor.unwrap_or_default())),
        "deprecated" => Some(
            "> **DEPRECATED — do not act on this record.** It has no successor. \
             Run `living-docs read` for what is in force."
                .to_string(),
        ),
        _ => None,
    }
}

fn superseded_callout(successor: &str) -> String {
    let number = number_prefix(successor);
    format!(
        "> **SUPERSEDED — do not act on this record.** Replaced by [{number}]({successor}). \
         Run `living-docs read` for what is in force."
    )
}

fn number_prefix(value: &str) -> &str {
    let end = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    &value[..end]
}

/// The first non-blank line of `body` when it opens a retired-record
/// callout, `None` otherwise.
pub fn leading(body: &str) -> Option<&str> {
    let first = body.lines().find(|line| !line.trim().is_empty())?;
    is_callout_line(first).then_some(first)
}

fn is_callout_line(line: &str) -> bool {
    line.starts_with(SUPERSEDED_MARKER) || line.starts_with(DEPRECATED_MARKER)
}

/// Rewrites `body` so it opens with `expected` (plus a trailing blank
/// line) when `Some`, or carries no callout at all when `None` —
/// stripping any callout `body` already opens with first. Idempotent: a
/// second call with the same `expected` returns the same string, and the
/// rest of `body` is otherwise byte-identical.
pub fn reconcile_body(body: &str, expected: Option<&str>) -> String {
    let stripped = strip_leading_callout(body);
    match expected {
        Some(line) => format!("{line}\n\n{stripped}"),
        None => stripped,
    }
}

fn strip_leading_callout(body: &str) -> String {
    let mut lines: Vec<&str> = body.split('\n').collect();
    let Some(index) = lines.iter().position(|line| !line.trim().is_empty()) else {
        return body.to_string();
    };
    if !is_callout_line(lines[index]) {
        return body.to_string();
    }
    lines.remove(index);
    if lines.get(index).is_some_and(|line| line.trim().is_empty()) {
        lines.remove(index);
    }
    lines.join("\n")
}

/// Resolves `superseded_by` (a zero-padded record number) to a sibling
/// filename in `path`'s own directory — a `<NNNN>-*.md` or `<NNNN>.md`
/// entry among `store.list`'s enumeration of that directory — falling
/// back to the bare number when no sibling matches.
pub fn successor_filename(store: &dyn DocStore, path: &Path, superseded_by: &str) -> String {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let Ok(candidates) = store.list(dir) else {
        return superseded_by.to_string();
    };
    find_sibling(&candidates, dir, superseded_by).unwrap_or_else(|| superseded_by.to_string())
}

fn find_sibling(candidates: &[std::path::PathBuf], dir: &Path, id: &str) -> Option<String> {
    let bare = dir.join(format!("{id}.md"));
    let prefix = format!("{id}-");
    candidates
        .iter()
        .find(|candidate| {
            candidate.parent() == Some(dir)
                && (candidate.as_path() == bare
                    || file_name(candidate).is_some_and(|name| name.starts_with(&prefix)))
        })
        .and_then(|candidate| file_name(candidate))
        .map(str::to_string)
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

/// Collapses the gap between the closing frontmatter fence and the body
/// down to a single newline before [`extract_record`] reads it. Without
/// this, a body already carrying the single blank line
/// [`to_canonical_markdown`] always inserts would extract with a leftover
/// leading newline — [`extract_record`] strips only one newline after the
/// fence — so re-serializing it would grow the gap by one newline on
/// every call instead of reaching a fixed point. Shared by every call site
/// that reads a record ahead of [`extract_record`] (`reconcile`, `fmt`).
pub(crate) fn normalize_frontmatter_gap(contents: &str) -> String {
    let Some(rest) = contents.strip_prefix("---\n") else {
        return contents.to_owned();
    };
    let Some(close_at) = rest.find("\n---") else {
        return contents.to_owned();
    };
    let fence_end = "---\n".len() + close_at + "\n---".len();
    let tail = contents[fence_end..].trim_start_matches('\n');
    format!("{}\n{tail}", &contents[..fence_end])
}

/// Reads the record at `path`, computes the callout its status calls for,
/// and rewrites the file through [`to_canonical_markdown`] only when the
/// reconciled body differs from what is already there. Returns whether it
/// wrote.
pub fn reconcile(store: &dyn DocStore, path: &Path) -> Result<bool, String> {
    let contents = store.read(path).map_err(|error| error.to_string())?;
    let contents = normalize_frontmatter_gap(&contents);
    let mut record = extract_record(path, &contents);

    let successor = record
        .superseded_by
        .as_deref()
        .map(|superseded_by| successor_filename(store, path, superseded_by));
    let expected_line = expected(record.status.as_deref(), successor.as_deref());
    let reconciled = reconcile_body(&record.body, expected_line.as_deref());

    if reconciled == record.body {
        return Ok(false);
    }

    record.body = reconciled;
    let canonical = to_canonical_markdown(&record);
    if let Some(key) = unwritable_key(&contents, &canonical) {
        return Err(refusal_message(path, &key));
    }
    store
        .write(path, &canonical)
        .map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests;
