//! The read-back guard of the shared frontmatter write path: after a draft is
//! built, every key it did not write must read as before, each written key
//! must read as its rendered line does alone, and the body must be unchanged.

use super::TailValue;
use crate::frontmatter::{frontmatter_block, parse_frontmatter, scalar_to_string};
use crate::record::{extract_record, kept_tag, record_numbers, ExtractedRecord};
use serde_yaml::{Mapping, Value};
use std::path::Path;

const SUPERSEDES_KEY: &str = "supersedes";
const TAGS_KEY: &str = "tags";

/// The first key whose drafted value does not read back as intended.
pub(super) fn unread_key(
    path: &Path,
    before: &str,
    draft: &str,
    fields: &[(&str, String)],
    expected: &[(&str, TailValue)],
) -> Option<String> {
    let (old, new) = (extract_record(path, before), extract_record(path, draft));
    let written = |key: &str| fields.iter().any(|(k, _)| *k == key);
    let unwritten_changed = frontmatter_keys(&old)
        .into_iter()
        .chain(frontmatter_keys(&new))
        .filter(|key| !written(key))
        .find(|key| key_value(&old, key) != key_value(&new, key));
    unwritten_changed
        .or_else(|| written_misread(path, &new, fields))
        .or_else(|| expected_misread(&new, expected))
        .or_else(|| body_changed(&old.body, &new.body).then(|| "body".to_string()))
}

/// Whether the draft's body differs from the original's by more than the
/// trailing newline the canonical pass appends to a record without one.
pub(super) fn body_changed(old: &str, new: &str) -> bool {
    old != new && (old.ends_with('\n') || new.strip_suffix('\n') != Some(old))
}

/// A written key whose drafted value differs from its rendered line read alone.
fn written_misread(
    path: &Path,
    draft: &ExtractedRecord,
    fields: &[(&str, String)],
) -> Option<String> {
    fields.iter().find_map(|(key, rendered)| {
        let alone = extract_record(path, &format!("---\n{key}: {rendered}\n---\n"));
        (key_value(draft, key) != key_value(&alone, key)).then(|| key.to_string())
    })
}

fn expected_misread(draft: &ExtractedRecord, expected: &[(&str, TailValue)]) -> Option<String> {
    expected
        .iter()
        .find(|(key, value)| key_value(draft, key).as_ref() != Some(value))
        .map(|(key, _)| key.to_string())
}

const TYPED_KEYS: [&str; 8] = [
    "type",
    "title",
    "description",
    "owner",
    "status",
    "supersedes",
    "superseded_by",
    "tags",
];

fn frontmatter_keys(record: &ExtractedRecord) -> Vec<String> {
    let tail = record.frontmatter_tail.iter().map(|(key, _)| key.clone());
    TYPED_KEYS
        .iter()
        .map(|key| key.to_string())
        .chain(tail)
        .collect()
}

/// A `supersedes` value as the guard compares it: a scalar for one entry, a
/// sequence for none or several, matching how the serializer renders it.
fn supersedes_value(supersedes: Option<&[String]>) -> Option<TailValue> {
    match supersedes? {
        [only] => Some(TailValue::Scalar(only.clone())),
        many => Some(TailValue::Sequence(many.to_vec())),
    }
}

/// The parsed value `record` holds for `key`, typed column or tail entry.
fn key_value(record: &ExtractedRecord, key: &str) -> Option<TailValue> {
    let scalar = |value: &str| Some(TailValue::Scalar(value.to_string()));
    let optional = |value: &Option<String>| value.as_deref().and_then(scalar);
    match key {
        "type" => scalar(&record.doc_type),
        "title" => scalar(&record.title),
        "description" => scalar(&record.description),
        "owner" => optional(&record.owner),
        "status" => optional(&record.status),
        "supersedes" => {
            supersedes_value(record.supersedes.as_deref()).or_else(|| tail_entry(record, key))
        }
        "superseded_by" => optional(&record.superseded_by),
        "tags" => Some(TailValue::Sequence(record.tags.clone())),
        _ => tail_entry(record, key),
    }
}

fn tail_entry(record: &ExtractedRecord, key: &str) -> Option<TailValue> {
    record
        .frontmatter_tail
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, value)| value.clone())
}

/// Whether `record` carries a `supersedes` value that is not a list of record
/// numbers, which a supersede must not append to.
pub(super) fn has_untyped_supersedes(record: &ExtractedRecord) -> bool {
    record.supersedes.is_none() && tail_entry(record, "supersedes").is_some()
}

const SCALAR_KEYS: [&str; 6] = [
    "type",
    "title",
    "description",
    "owner",
    "status",
    "superseded_by",
];

/// The first frontmatter key of `before` that its canonical form `canonical`
/// does not read back as: scalar keys compare as strings, list items with their
/// YAML type, tags in canonical order. The typed identity and a value the
/// canonical pass drops by design (null, empty, a mapping) are not compared.
/// A blank scalar is not compared either: the canonical pass reseeds a blank
/// title. `None` when `before` has no readable frontmatter.
pub(crate) fn unwritable_key(before: &str, canonical: &str) -> Option<String> {
    let old = frontmatter_mapping(before)?;
    let new = frontmatter_mapping(canonical).unwrap_or_default();
    old.iter().find_map(|(key, was)| {
        let key = key.as_str()?;
        (compared(key, was) && !reads_same(key, was, new.get(key))).then(|| key.to_owned())
    })
}

/// The refusal text for a record whose `key` has no canonical form.
pub(crate) fn refusal_message(path: &Path, key: &str) -> String {
    format!(
        "{}: '{key}' cannot be written canonically; edit the file by hand",
        path.display()
    )
}

fn frontmatter_mapping(contents: &str) -> Option<Mapping> {
    match parse_frontmatter(frontmatter_block(contents)?)? {
        Value::Mapping(mapping) => Some(mapping),
        _ => None,
    }
}

fn compared(key: &str, was: &Value) -> bool {
    match key {
        "number" | "concept_id" => false,
        TAGS_KEY => was.as_sequence().is_some_and(|items| !items.is_empty()),
        _ if SCALAR_KEYS.contains(&key) => {
            scalar_to_string(was).is_some_and(|text| !text.trim().is_empty())
        }
        _ => match was {
            Value::String(text) => !text.is_empty(),
            Value::Sequence(items) => !items.is_empty(),
            Value::Null | Value::Mapping(_) | Value::Tagged(_) => false,
            _ => true,
        },
    }
}

fn reads_same(key: &str, was: &Value, now: Option<&Value>) -> bool {
    let Some(now) = now else { return false };
    if !was.is_sequence() && !now.is_sequence() {
        return scalar_to_string(was) == scalar_to_string(now);
    }
    let (mut was, mut now) = (items(was), items(now));
    if key == TAGS_KEY {
        for list in [&mut was, &mut now] {
            list.retain(kept_tag);
            list.sort_by_key(|item| format!("{item:?}"));
        }
    }
    match (record_numbers(&was), record_numbers(&now)) {
        (Some(a), Some(b)) if key == SUPERSEDES_KEY => a == b,
        _ => was == now,
    }
}

fn items(value: &Value) -> Vec<Value> {
    match value {
        Value::Sequence(items) => items.clone(),
        other => vec![other.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record_with(supersedes: &str) -> String {
        format!("---\ntype: ADR\ntitle: T\ndescription: d\nstatus: Accepted\n{supersedes}\n---\n\nbody\n")
    }

    #[test]
    fn a_draft_that_changes_a_carried_supersedes_value_is_refused() {
        let path = Path::new("/bundle/adr/0004-x.md");
        let before = record_with("supersedes: [0001, \"\"]");
        let draft = record_with("supersedes: [0001, \"changed\"]");
        let written = [("description", "d".to_owned())];

        let unread = unread_key(path, &before, &draft, &written, &[]);

        assert_eq!(unread.as_deref(), Some("supersedes"));
    }

    fn block(extra: &str) -> String {
        format!("---\ntype: ADR\ntitle: T\n{extra}\n---\n\nbody\n")
    }

    #[test]
    fn a_canonical_draft_whose_list_item_reads_back_with_another_type_is_refused() {
        let before = block("aliases: [a, \"1\"]");
        for draft in ["aliases: [a, 1]", "aliases: [a]", "aliases: [a, \"2\"]"] {
            let unwritable = unwritable_key(&before, &block(draft));

            assert_eq!(unwritable.as_deref(), Some("aliases"), "{draft}");
        }
    }

    #[test]
    fn a_canonical_draft_that_keeps_every_list_item_and_type_is_accepted() {
        let before = block("aliases: [a, \"1\", ~, 2]\ntags: [b, a]");
        let draft = block("aliases: [a, \"1\", ~, 2]\ntags: [a, b]");

        assert_eq!(unwritable_key(&before, &draft), None);
    }

    #[test]
    fn a_nested_list_item_has_no_canonical_form() {
        let before = block("tags: [a, [b]]");
        let draft = block("tags: [a]");

        assert_eq!(unwritable_key(&before, &draft).as_deref(), Some("tags"));
    }

    #[test]
    fn a_draft_that_keeps_a_carried_supersedes_value_is_accepted() {
        let path = Path::new("/bundle/adr/0004-x.md");
        let before = record_with("supersedes: [0001, \"\"]");
        let written = [("description", "d".to_owned())];

        assert_eq!(unread_key(path, &before, &before, &written, &[]), None);
    }
}
