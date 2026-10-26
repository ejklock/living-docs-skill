//! Canonical serialization: the inverse of [`extract_record`](super::extract_record),
//! turning an [`ExtractedRecord`] back into canonical markdown, plus the
//! YAML scalar and flow-list rendering it shares with the frontmatter writers.

use super::{ExtractedRecord, TailValue};
use crate::frontmatter::scalar_to_string;
use serde_yaml::Value;

const STATUS_KEY: &str = "status";
const TRACKER_KEY: &str = "tracker";
const TIMESTAMP_KEY: &str = "timestamp";
pub(super) const SUPERSEDES_KEY: &str = "supersedes";
pub(super) const TAGS_KEY: &str = "tags";

/// Reconstructs `record` as canonical markdown: a `---`-fenced frontmatter
/// block in the fixed field order — `type`, `title`, `description`, `owner`
/// (if present), `status` (if present), `supersedes`, `superseded_by`,
/// `tags`, the remaining frontmatter tail by ascending ordinal, `tracker`
/// (if present), then `timestamp` (if present) — followed by a blank line
/// and the body. The typed identity (`number`/`concept_id`) is never
/// emitted: it is carried by the record's path, not its frontmatter.
/// Re-parsing the output through [`extract_record`](super::extract_record) reproduces every field
/// `extract_record` reads: the serializer's canonical order is a fixed
/// point for a record already shaped this way, not a byte-for-byte match of
/// arbitrary hand-authored source.
pub fn to_canonical_markdown(record: &ExtractedRecord) -> String {
    let mut lines = Vec::new();
    lines.push(format!("type: {}", format_scalar(&record.doc_type)));
    lines.push(format!("title: {}", format_scalar(&record.title)));
    lines.push(format!(
        "description: {}",
        format_scalar(&record.description)
    ));
    push_optional(&mut lines, "owner", record.owner.as_deref());
    push_optional(&mut lines, STATUS_KEY, record.status.as_deref());
    push_supersedes(&mut lines, record);
    push_optional(&mut lines, "superseded_by", record.superseded_by.as_deref());
    push_tags(&mut lines, record);
    push_remaining_tail(&mut lines, record);
    push_tail_key(&mut lines, record, TRACKER_KEY);
    push_tail_key(&mut lines, record, TIMESTAMP_KEY);

    format!("---\n{}\n---\n\n{}", lines.join("\n"), record.body)
}

/// Renders `supersedes` as a scalar when it holds one record number, so a
/// record replacing a single record keeps its plain `supersedes: NNNN` line, and
/// as a flow list for zero or several. A value that is not a list of record
/// numbers rides in the tail and is written back as a flow list of its items.
fn push_supersedes(lines: &mut Vec<String>, record: &ExtractedRecord) {
    match record.supersedes.as_deref() {
        Some([only]) => lines.push(format!("supersedes: {}", format_scalar(only))),
        Some(many) => lines.push(format!("supersedes: {}", flow_strings(many))),
        None => push_tail_key(lines, record, SUPERSEDES_KEY),
    }
}

/// The entries of a sequence when every one is a record number.
pub(crate) fn record_numbers(entries: &[Value]) -> Option<Vec<String>> {
    let number = |entry| scalar_to_string(entry).filter(|text| text.trim().parse::<u32>().is_ok());
    entries.iter().map(number).collect()
}

fn push_optional(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{key}: {}", format_scalar(value)));
    }
}

/// Renders `tags` as a flow sequence, sorted for deterministic output — the
/// `tags`/`record_tags` join carries no ordinal, so the original frontmatter
/// order is not recoverable and alphabetical order is the canonical one. A tag
/// list holding a number or boolean item is written from its typed tail entry,
/// so those items stay plain.
fn push_tags(lines: &mut Vec<String>, record: &ExtractedRecord) {
    if let Some(typed) = typed_tags(record) {
        lines.push(format!("tags: {}", flow_values(&typed)));
    } else if !record.tags.is_empty() {
        let mut sorted = record.tags.clone();
        sorted.sort();
        lines.push(format!("tags: {}", flow_strings(&sorted)));
    }
}

/// The typed tag items in sorted order, when the tail entry still agrees
/// with `record.tags`.
fn typed_tags(record: &ExtractedRecord) -> Option<Vec<Value>> {
    let TailValue::Typed(items) = tail_value(record, TAGS_KEY)? else {
        return None;
    };
    let mut kept: Vec<Value> = items
        .iter()
        .filter(|item| kept_tag(item))
        .cloned()
        .collect();
    kept.sort_by_key(|item| scalar_to_string(item).unwrap_or_default());
    let mut names: Vec<String> = kept.iter().filter_map(scalar_to_string).collect();
    let mut tags = record.tags.clone();
    names.sort();
    tags.sort();
    (names == tags).then_some(kept)
}

/// A tag item the tag column keeps: neither null nor an empty string.
pub(crate) fn kept_tag(item: &Value) -> bool {
    !matches!(item, Value::Null) && !matches!(item, Value::String(text) if text.is_empty())
}

fn push_tail_key(lines: &mut Vec<String>, record: &ExtractedRecord, key: &str) {
    if let Some(value) = tail_value(record, key) {
        push_tail_value(lines, key, value);
    }
}

/// Every tail entry except `status`/`tracker`/`timestamp`, which are pulled
/// to their own fixed positions elsewhere in the fixed field order, in the
/// ordinal order they arrive in `record.frontmatter_tail`.
fn push_remaining_tail(lines: &mut Vec<String>, record: &ExtractedRecord) {
    for (key, value) in &record.frontmatter_tail {
        if is_special_tail_key(key) {
            continue;
        }
        push_tail_value(lines, key, value);
    }
}

/// Renders one tail entry as `key: value`, or `key: [a, b]` for a sequence.
fn push_tail_value(lines: &mut Vec<String>, key: &str, value: &TailValue) {
    match value {
        TailValue::Scalar(scalar) => lines.push(format!("{key}: {}", format_scalar(scalar))),
        TailValue::Sequence(items) => lines.push(format!("{key}: {}", flow_strings(items))),
        TailValue::Typed(items) => lines.push(format!("{key}: {}", flow_values(items))),
    }
}

fn tail_value<'a>(record: &'a ExtractedRecord, key: &str) -> Option<&'a TailValue> {
    record
        .frontmatter_tail
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

fn is_special_tail_key(key: &str) -> bool {
    [TRACKER_KEY, TIMESTAMP_KEY, SUPERSEDES_KEY, TAGS_KEY].contains(&key)
}

/// Renders `items` as a flow sequence (`[a, b]`), each quoted only when
/// [`needs_quoting`] says so.
pub(crate) fn format_flow_list<S: AsRef<str>>(items: &[S]) -> String {
    let render = |item: &str| {
        if needs_quoting(item) {
            quoted(item)
        } else {
            item.to_owned()
        }
    };
    let items: Vec<String> = items.iter().map(|i| render(i.as_ref())).collect();
    format!("[{}]", items.join(", "))
}

/// Renders string `items` as a flow sequence whose every item reads back as
/// that string, each quoted by [`quote_flow`].
fn flow_strings<S: AsRef<str>>(items: &[S]) -> String {
    let items: Vec<String> = items.iter().map(|i| quote_flow(i.as_ref())).collect();
    format!("[{}]", items.join(", "))
}

/// Renders typed items as a flow sequence: a null as `~`, a number or boolean
/// plain, a string through [`quote_flow`]. A nested item has no flow form and
/// renders empty; the read-back guard refuses a record that held one.
fn flow_values(items: &[Value]) -> String {
    let render = |item: &Value| match item {
        Value::Null => "~".to_owned(),
        Value::String(text) => quote_flow(text),
        other => scalar_to_string(other).unwrap_or_default(),
    };
    let items: Vec<String> = items.iter().map(render).collect();
    format!("[{}]", items.join(", "))
}

/// A string as one flow-sequence item: quoted by the `needs_quoting` rules,
/// and also when its plain form, read back inside a flow sequence, would
/// give anything but this string (a flow indicator, a line break, a null, a
/// boolean, a number).
fn quote_flow(value: &str) -> String {
    if needs_quoting(value) || !reads_back_plain(value) {
        quoted(value)
    } else {
        value.to_owned()
    }
}

fn reads_back_plain(value: &str) -> bool {
    match serde_yaml::from_str::<Value>(&format!("[{value}]")) {
        Ok(Value::Sequence(items)) => items == [Value::String(value.to_owned())],
        _ => false,
    }
}

const YAML_INDICATOR_PREFIXES: &str = "!&*-?|>%@`\"'#,[]{}";

/// Renders `value` as a plain YAML scalar when safe, or a double-quoted
/// scalar (with `\`/`"` escaped) when a plain scalar would be ambiguous —
/// empty, starting with a YAML indicator character, containing `": "` or a
/// ` #` comment opener, ending in `:`, or carrying surrounding whitespace.
/// A value whose plain form the reader would return as a different string (a
/// null, or a number such as `1.50` that reads as `1.5`) is quoted too.
pub(crate) fn format_scalar(value: &str) -> String {
    if needs_quoting(value) || !reads_back_as_block_scalar(value) {
        quoted(value)
    } else {
        value.to_owned()
    }
}

/// Whether `value` written plain as a block-mapping value reads back, through
/// the reader's string form, as the same string.
fn reads_back_as_block_scalar(value: &str) -> bool {
    match serde_yaml::from_str::<Value>(&format!("k: {value}")) {
        Ok(Value::Mapping(map)) => {
            map.get("k").and_then(scalar_to_string).as_deref() == Some(value)
        }
        _ => false,
    }
}

fn quoted(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped.replace('\n', "\\n"))
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value.starts_with(|c: char| YAML_INDICATOR_PREFIXES.contains(c))
        || value.contains(": ")
        || value.ends_with(':')
        || value.contains(" #")
        || value.contains("\t#")
        || value.trim() != value
}
