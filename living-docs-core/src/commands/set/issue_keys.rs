use super::resolve_spec;
use crate::record::canonical::needs_quoting;
use crate::record::{format_flow_list, format_scalar, TailValue};
use crate::store::DocStore;
use serde_yaml::Value;
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
    let rendered = format_flow_list(&items);
    let intended = if key == "blocked_by" {
        refuse_unreadable(key, &items)?;
        typed_items(&items)
    } else {
        TailValue::Sequence(items)
    };
    Ok((rendered, intended))
}

/// Types the `blocked_by` items for the read-back check: a decimal number
/// (ASCII digits only) the record reads as a YAML number (such as `12`) is
/// expected back typed, any other item (`0012`, `ref ab-77`, `-3`, `#21`) as
/// the text given. Items the writer leaves plain and YAML would read as
/// another value are refused beforehand by `refuse_unreadable`.
fn typed_items(items: &[String]) -> TailValue {
    let read = |item: &String| match serde_yaml::from_str::<Value>(item) {
        Ok(number @ Value::Number(_)) if item.bytes().all(|b| b.is_ascii_digit()) => number,
        _ => Value::String(item.clone()),
    };
    let typed: Vec<Value> = items.iter().map(read).collect();
    if typed.iter().any(Value::is_number) {
        TailValue::Typed(typed)
    } else {
        TailValue::Sequence(items.to_vec())
    }
}

/// Refuses the first item the writer leaves plain that YAML reads as another
/// type than the text given (a boolean, null, or a number that is not a plain
/// integer: `+4`, `1.5`, `0x10`, `1e3`). An item the writer quotes (`#21`,
/// `-0`) reads back as its text and is never refused; nor is a signed integer
/// such as `-3`.
fn refuse_unreadable(key: &str, items: &[String]) -> Result<(), String> {
    let changes = |item: &&String| {
        if needs_quoting(item) {
            return false;
        }
        match serde_yaml::from_str::<Value>(item) {
            Ok(Value::Null | Value::Bool(_)) => true,
            Ok(Value::Number(n)) => !((n.is_i64() || n.is_u64()) && n.to_string() == **item),
            _ => false,
        }
    };
    match items.iter().find(changes) {
        Some(item) => Err(format!(
            "'{key}': '{item}' would read back as a different value; quote-free YAML cannot hold it, so write it in the issue body or as plain text"
        )),
        None => Ok(()),
    }
}
