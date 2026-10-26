//! Extraction of the frontmatter keys that have no typed column, the
//! inverse of the tail the canonical writer emits.

use super::canonical::{record_numbers, SUPERSEDES_KEY, TAGS_KEY};
use super::{TailValue, TYPED_FRONTMATTER_KEYS};
use crate::frontmatter::scalar_to_string;
use serde_yaml::Value;

/// Every frontmatter key with no typed column, in source encounter order
/// (a decision the registry owns), plus a `supersedes` list that is not all
/// record numbers and a `tags` list holding a number or boolean item, which
/// keep their item types in a [`TailValue::Typed`] entry. Relies on
/// `serde_yaml::Mapping` preserving the document's key order. A nested item
/// or a nested mapping value is dropped here; the read-back guard refuses a
/// record that held one.
pub(super) fn extract_frontmatter_tail(frontmatter: Option<&Value>) -> Vec<(String, TailValue)> {
    let Some(mapping) = frontmatter.and_then(Value::as_mapping) else {
        return Vec::new();
    };
    mapping
        .iter()
        .filter_map(|(key, value)| {
            let key = key.as_str()?;
            let value = carried(key, value).then(|| tail_value_from_yaml(value))??;
            Some((key.to_owned(), value))
        })
        .collect()
}

fn carried(key: &str, value: &Value) -> bool {
    let has = |test: fn(&Value) -> bool| value.as_sequence().is_some_and(|s| s.iter().any(test));
    match key {
        SUPERSEDES_KEY => value
            .as_sequence()
            .is_some_and(|items| record_numbers(items).is_none()),
        TAGS_KEY => has(|item| matches!(item, Value::Number(_) | Value::Bool(_))),
        _ => !TYPED_FRONTMATTER_KEYS.contains(&key),
    }
}

/// One frontmatter value as a [`TailValue`]: a scalar (an empty string
/// included, a null dropped), a sequence of
/// non-empty strings, or a [`TailValue::Typed`] sequence when an item is a
/// number, boolean, null or empty string. `None` for a mapping.
fn tail_value_from_yaml(value: &Value) -> Option<TailValue> {
    let Some(sequence) = value.as_sequence() else {
        return match value {
            Value::String(text) => Some(TailValue::Scalar(text.clone())),
            other => scalar_to_string(other).map(TailValue::Scalar),
        };
    };
    let flat: Vec<&Value> = sequence
        .iter()
        .filter(|item| {
            !matches!(
                item,
                Value::Sequence(_) | Value::Mapping(_) | Value::Tagged(_)
            )
        })
        .collect();
    let text = |item: &&Value| matches!(item, Value::String(s) if !s.is_empty());
    if flat.iter().all(text) {
        return Some(TailValue::Sequence(
            flat.iter().filter_map(|i| scalar_to_string(i)).collect(),
        ));
    }
    Some(TailValue::Typed(flat.into_iter().cloned().collect()))
}
