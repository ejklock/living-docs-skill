//! Canonical markdown serializer — the inverse of
//! [`crate::record::extract_record`] (ADR 0007 decision 3, issue 0006 slice
//! 0006-B). Pure: takes an already-assembled [`ExtractedRecord`] and renders
//! frontmatter text in the fixed canonical field order; no I/O, no database
//! access. `crate::lib::DbDocStore::read` assembles the `ExtractedRecord`
//! from the read-model and hands it here. Emits no `number:`/`concept_id:`
//! frontmatter line (issue 0006 slice 0006-C1, lesson 3706): identity is
//! carried by the record's path alone, never by a frontmatter field.

use crate::record::ExtractedRecord;

const STATUS_KEY: &str = "status";
const TRACKER_KEY: &str = "tracker";
const TIMESTAMP_KEY: &str = "timestamp";

/// Reconstructs `record` as canonical markdown: a `---`-fenced frontmatter
/// block in the fixed field order — `type`, `title`, `description`,
/// `status` (if present), `supersedes`, `superseded_by`, `tags`,
/// the remaining frontmatter tail by ascending ordinal, `tracker` (if
/// present), then `timestamp` (if present) — followed by a blank line and
/// the body. The typed identity (`number`/`concept_id`) is never emitted:
/// it is carried by the record's path, not its frontmatter. Re-parsing the
/// output through [`crate::record::extract_record`] reproduces every field
/// `extract_record` reads (ADR 0007 decision 3): the serializer's canonical
/// order is a fixed point for a record already shaped this way, not a
/// byte-for-byte match of arbitrary hand-authored source.
pub fn to_canonical_markdown(record: &ExtractedRecord) -> String {
    let mut lines = Vec::new();
    lines.push(format!("type: {}", format_scalar(&record.doc_type)));
    lines.push(format!("title: {}", format_scalar(&record.title)));
    lines.push(format!(
        "description: {}",
        format_scalar(&record.description)
    ));
    push_optional(&mut lines, STATUS_KEY, record.status.as_deref());
    push_optional(&mut lines, "supersedes", record.supersedes.as_deref());
    push_optional(&mut lines, "superseded_by", record.superseded_by.as_deref());
    push_tags(&mut lines, &record.tags);
    push_remaining_tail(&mut lines, record);
    push_tail_key(&mut lines, record, TRACKER_KEY);
    push_tail_key(&mut lines, record, TIMESTAMP_KEY);

    format!("---\n{}\n---\n\n{}", lines.join("\n"), record.body)
}

fn push_optional(lines: &mut Vec<String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{key}: {}", format_scalar(value)));
    }
}

/// Renders `tags` as a flow sequence, sorted for deterministic output — the
/// `tags`/`record_tags` join carries no ordinal, so the original frontmatter
/// order is not recoverable and alphabetical order is the canonical one.
fn push_tags(lines: &mut Vec<String>, tags: &[String]) {
    if tags.is_empty() {
        return;
    }
    let mut sorted = tags.to_vec();
    sorted.sort();
    let rendered: Vec<String> = sorted.iter().map(|tag| format_scalar(tag)).collect();
    lines.push(format!("tags: [{}]", rendered.join(", ")));
}

fn push_tail_key(lines: &mut Vec<String>, record: &ExtractedRecord, key: &str) {
    if let Some(value) = tail_value(record, key) {
        lines.push(format!("{key}: {}", format_scalar(value)));
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
        lines.push(format!("{key}: {}", format_scalar(value)));
    }
}

fn tail_value<'a>(record: &'a ExtractedRecord, key: &str) -> Option<&'a str> {
    record
        .frontmatter_tail
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn is_special_tail_key(key: &str) -> bool {
    key == TRACKER_KEY || key == TIMESTAMP_KEY
}

const YAML_INDICATOR_PREFIXES: &str = "!&*-?|>%@`\"'#,[]{}";

/// Renders `value` as a plain YAML scalar when safe, or a double-quoted
/// scalar (with `\`/`"` escaped) when a plain scalar would be ambiguous —
/// empty, starting with a YAML indicator character, containing `": "`,
/// ending in `:`, or carrying leading/trailing whitespace.
fn format_scalar(value: &str) -> String {
    if needs_quoting(value) {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value.starts_with(|c: char| YAML_INDICATOR_PREFIXES.contains(c))
        || value.contains(": ")
        || value.ends_with(':')
        || value.trim() != value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{extract_record, CONCEPT_IDENTITY_KIND, NUMBER_IDENTITY_KIND};
    use std::path::Path;

    fn numbered_record() -> ExtractedRecord {
        ExtractedRecord {
            doc_type: "ADR".to_owned(),
            number: Some(1),
            concept_id: None,
            identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
            title: "Tailed Decision".to_owned(),
            description: "d.".to_owned(),
            body: "# 0001. Tailed Decision\n\nBody.\n".to_owned(),
            supersedes: Some("0001".to_owned()),
            superseded_by: None,
            tags: vec!["caching".to_owned(), "performance".to_owned()],
            status: Some("Accepted".to_owned()),
            frontmatter_tail: vec![
                ("labels".to_owned(), "important".to_owned()),
                ("blocked_by".to_owned(), "0002".to_owned()),
                ("tracker".to_owned(), "JIRA-42".to_owned()),
                ("timestamp".to_owned(), "2026-07-17T00:00:00Z".to_owned()),
            ],
        }
    }

    #[test]
    fn to_canonical_markdown_emits_the_fixed_field_order() {
        let record = numbered_record();

        let markdown = to_canonical_markdown(&record);

        assert_eq!(
            markdown,
            "---\n\
             type: ADR\n\
             title: Tailed Decision\n\
             description: d.\n\
             status: Accepted\n\
             supersedes: 0001\n\
             tags: [caching, performance]\n\
             labels: important\n\
             blocked_by: 0002\n\
             tracker: JIRA-42\n\
             timestamp: 2026-07-17T00:00:00Z\n\
             ---\n\
             \n\
             # 0001. Tailed Decision\n\n\
             Body.\n"
        );
        assert!(!markdown.contains("number:"));
    }

    /// Asserts the round-trip fields ADR 0007 decision 3 / issue 0006 AC B1
    /// name as the fixed point: `doc_type`, `title`, `description`, the
    /// typed identity, `supersedes`/`superseded_by`, `tags`, and the
    /// frontmatter tail. `body` is deliberately excluded — the canonical
    /// serializer inserts a blank line before it (AC B3), so a source body
    /// that already started with one round-trips with an extra line; AC B1
    /// does not list `body` among the fields the fixed point covers.
    fn assert_round_trips(reparsed: &ExtractedRecord, original: &ExtractedRecord) {
        assert_eq!(reparsed.doc_type, original.doc_type);
        assert_eq!(reparsed.title, original.title);
        assert_eq!(reparsed.description, original.description);
        assert_eq!(reparsed.number, original.number);
        assert_eq!(reparsed.concept_id, original.concept_id);
        assert_eq!(reparsed.identity_kind, original.identity_kind);
        assert_eq!(reparsed.supersedes, original.supersedes);
        assert_eq!(reparsed.superseded_by, original.superseded_by);
        assert_eq!(reparsed.tags, original.tags);
        assert_eq!(reparsed.status, original.status);
        assert_eq!(reparsed.frontmatter_tail, original.frontmatter_tail);
    }

    #[test]
    fn to_canonical_markdown_round_trips_a_numbered_record_through_extract_record() {
        let record = numbered_record();

        let markdown = to_canonical_markdown(&record);
        let reparsed = extract_record(Path::new("adr/0001-tailed.md"), &markdown);

        assert_round_trips(&reparsed, &record);
    }

    #[test]
    fn to_canonical_markdown_round_trips_a_concept_record_through_extract_record() {
        let record = ExtractedRecord {
            doc_type: "Glossary".to_owned(),
            number: None,
            concept_id: Some("glossary/findability".to_owned()),
            identity_kind: CONCEPT_IDENTITY_KIND.to_owned(),
            title: "Findability".to_owned(),
            description: "The ease of locating a doc.".to_owned(),
            body: "# Findability\n\nBody.\n".to_owned(),
            supersedes: None,
            superseded_by: None,
            tags: vec!["glossary".to_owned()],
            status: Some("Active".to_owned()),
            frontmatter_tail: Vec::new(),
        };

        let markdown = to_canonical_markdown(&record);
        let reparsed = extract_record(Path::new("glossary/findability.md"), &markdown);

        assert_round_trips(&reparsed, &record);
    }

    #[test]
    fn to_canonical_markdown_omits_absent_optional_fields() {
        let record = ExtractedRecord {
            doc_type: "ADR".to_owned(),
            number: None,
            concept_id: None,
            identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
            title: "No Extras".to_owned(),
            description: String::new(),
            body: "Body.\n".to_owned(),
            supersedes: None,
            superseded_by: None,
            tags: Vec::new(),
            status: None,
            frontmatter_tail: Vec::new(),
        };

        let markdown = to_canonical_markdown(&record);

        assert_eq!(
            markdown,
            "---\ntype: ADR\ntitle: No Extras\ndescription: \"\"\n---\n\nBody.\n"
        );
        assert!(!markdown.contains("number:"));
        assert!(!markdown.contains("supersedes:"));
        assert!(!markdown.contains("tags:"));
    }

    #[test]
    fn format_scalar_quotes_a_value_containing_a_colon_space_and_round_trips_it() {
        let record = ExtractedRecord {
            doc_type: "ADR".to_owned(),
            number: Some(2),
            concept_id: None,
            identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
            title: "Caching: A Deep Dive".to_owned(),
            description: "d.".to_owned(),
            body: "Body.\n".to_owned(),
            supersedes: None,
            superseded_by: None,
            tags: Vec::new(),
            status: None,
            frontmatter_tail: Vec::new(),
        };

        let markdown = to_canonical_markdown(&record);

        assert!(markdown.contains("title: \"Caching: A Deep Dive\""));
        let reparsed = extract_record(Path::new("adr/0002-caching.md"), &markdown);
        assert_eq!(reparsed.title, "Caching: A Deep Dive");
    }

    /// Asserts `status` is read from [`ExtractedRecord::status`] rather than
    /// the frontmatter tail (issue 0008, ADR 0015, S1 round 2): the typed
    /// field must still round-trip to a `status:` frontmatter line even
    /// though `frontmatter_tail` never carries a `status` entry.
    #[test]
    fn to_canonical_markdown_emits_status_from_the_typed_field() {
        let record = ExtractedRecord {
            doc_type: "ADR".to_owned(),
            number: Some(3),
            concept_id: None,
            identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
            title: "Typed Status".to_owned(),
            description: "d.".to_owned(),
            body: "Body.\n".to_owned(),
            supersedes: None,
            superseded_by: None,
            tags: Vec::new(),
            status: Some("Accepted".to_owned()),
            frontmatter_tail: Vec::new(),
        };

        let markdown = to_canonical_markdown(&record);

        assert!(markdown.contains("status: Accepted"));
        let reparsed = extract_record(Path::new("adr/0003-typed-status.md"), &markdown);
        assert_eq!(reparsed.status, Some("Accepted".to_owned()));
    }
}
