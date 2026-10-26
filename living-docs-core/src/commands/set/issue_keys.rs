use super::resolve_spec;
use crate::record::{format_flow_list, format_scalar, TailValue};
use crate::store::DocStore;
use std::path::Path;

/// The one doc type whose records carry the keys this module writes.
const ISSUE_TYPE: &str = "Issue";

/// Builds the frontmatter field for `tracker` (one scalar), `labels` or
/// `blocked_by` (a comma-separated value written as a flow list). The record
/// at `path` must be an issue, so the keys are refused on every other doc
/// type. Nothing is written here; the caller writes the returned field and
/// hands the returned value to the write path, which refuses the write unless
/// the key reads back as exactly that value.
pub(super) fn field(
    store: &dyn DocStore,
    path: &Path,
    key: &'static str,
    value: &str,
) -> Result<(&'static str, String, TailValue), String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let spec = resolve_spec(path, &contents)?;
    if spec.frontmatter != ISSUE_TYPE {
        return Err(format!(
            "'{key}' is settable only on an {ISSUE_TYPE} record, not on {}",
            spec.frontmatter
        ));
    }
    let (rendered, intended) = if key == "tracker" {
        tracker_value(value)?
    } else {
        list_value(key, value)?
    };
    Ok((key, rendered, intended))
}

fn tracker_value(value: &str) -> Result<(String, TailValue), String> {
    let value = value.trim();
    if value.is_empty() || value.contains(['\n', '\r']) {
        return Err("'tracker' takes one non-empty line of text".to_string());
    }
    Ok((format_scalar(value), TailValue::Scalar(value.to_string())))
}

/// Splits `value` on commas into trimmed, non-empty items and renders the
/// flow list the canonical serializer emits (`[a, b]`, `[]` when empty).
fn list_value(key: &str, value: &str) -> Result<(String, TailValue), String> {
    if value.contains(['[', ']', '{', '}', '"', '\n', '\r']) {
        return Err(format!(
            "'{key}' items cannot contain brackets, braces, quotes or line breaks; give plain comma-separated items"
        ));
    }
    let items: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    Ok((format_flow_list(&items), TailValue::Sequence(items)))
}
