//! Canonical record model, shared by every backend and front (ADR 0019
//! slice S1): frontmatter/body extraction ([`extract_record`]) and its
//! inverse, canonical serialization ([`to_canonical_markdown`]). Moved here
//! verbatim from `db-store` so a future non-db-store backend can reuse the
//! same model without depending on `db-store` (ADR 0004, issue 0002 slice
//! S2b; supersedes/superseded_by/tags parsing ADR 0005 issue 0005 slice
//! 0005-B; dual typed identity + EAV frontmatter tail ADR 0007 issue 0006
//! slice 0006-A; identity sourced from the record's path rather than
//! frontmatter, issue 0006 slice 0006-C1; canonical serializer issue 0006
//! slice 0006-B). Every function here takes already-read file contents or an
//! already-assembled [`ExtractedRecord`]; none touches the filesystem or a
//! database. `db_store::record`/`db_store::serialize` re-export this
//! module's public items so `db_store::sync::sync_project` and
//! `db_store::DbDocStore::read` keep resolving unchanged.
//!
//! [`ExtractedRecord`] and the [`NUMBER_IDENTITY_KIND`]/
//! [`CONCEPT_IDENTITY_KIND`] constants are shared between [`extract_record`]
//! and [`to_canonical_markdown`], which is this module's inverse: whatever
//! [`extract_record`] parses out of a `.md` file, `to_canonical_markdown`
//! reconstructs from an `ExtractedRecord` back into one.

use std::path::Path;

use serde_yaml::Value;

use crate::frontmatter::{
    frontmatter_block, parse_frontmatter, read_scalar_strict, scalar_to_string,
};

/// The `identity_kind` discriminator for a sequentially numbered doc
/// (`NNNN`, e.g. adr/bdr/prd/issue).
pub const NUMBER_IDENTITY_KIND: &str = "number";

/// The `identity_kind` discriminator for a path-identified OKF concept doc.
pub const CONCEPT_IDENTITY_KIND: &str = "concept";

/// Frontmatter `type` values that carry a sequential `NNNN` identity rather
/// than a `concept_id` (ADR 0007 decision 2).
const NUMBERED_DOC_TYPES: [&str; 4] = ["adr", "bdr", "prd", "issue"];

/// Frontmatter keys that already have a universal typed column or dedicated
/// handling elsewhere, and therefore never land in the EAV
/// [`ExtractedRecord::frontmatter_tail`] (ADR 0007 decision 1).
const TYPED_FRONTMATTER_KEYS: [&str; 9] = [
    "type",
    "title",
    "description",
    "number",
    "concept_id",
    "supersedes",
    "superseded_by",
    "tags",
    "status",
];

/// The fields extracted from a doc record's raw contents, ready to insert
/// into the `records` table. `identity_kind` is derived from `doc_type`
/// (ADR 0007 decision 2): a numbered type (adr/bdr/prd/issue) carries
/// `number` — the record's path's filename `NNNN` prefix — with
/// `concept_id` left `None`; every other type carries `concept_id` — the
/// record's path with a trailing `.md` removed — with `number` left `None`
/// (issue 0006 slice 0006-C1: identity is sourced from the path, never from
/// a `number:`/`concept_id:` frontmatter key — real OKF docs carry neither).
/// `supersedes`/`superseded_by` carry the raw `NNNN` frontmatter value
/// (unresolved to a record id — that
/// resolution happens against a project's other records in
/// `db_store::sync::sync_project`); `tags` is the frontmatter's `tags`
/// sequence, empty when absent. `status` is the frontmatter `status:`
/// value, `None` when the key is absent (issue 0008, ADR 0015, S1).
/// `frontmatter_tail` is every remaining frontmatter key with no typed
/// column, in source encounter order, ready to insert into
/// `frontmatter_fields` with the index as `ordinal`. Each entry's
/// [`TailValue`] carries either a single scalar or an ordered sequence of
/// scalars (ADR 0007's EAV tail extended for list-valued keys like
/// `labels:`/`blocked_by:`, ADR 0019 slice S3b) — a nested mapping or a
/// sequence element that is itself non-scalar stays outside the contract,
/// exactly as a non-scalar value already did before this extension.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedRecord {
    pub doc_type: String,
    pub number: Option<i32>,
    pub concept_id: Option<String>,
    pub identity_kind: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub tags: Vec<String>,
    pub status: Option<String>,
    pub frontmatter_tail: Vec<(String, TailValue)>,
}

/// A single [`ExtractedRecord::frontmatter_tail`] entry's value: either the
/// scalar `key: value` shape the tail always carried, or the ordered
/// sequence a `key: [a, b]` YAML flow list parses into (ADR 0007's
/// lossless-export contract extended to list-valued tail keys, ADR 0019
/// slice S3b). [`to_canonical_markdown`] re-emits a `Sequence` in the same
/// flow style tags already use, quoting each element with the same
/// [`format_scalar`] rules a `Scalar` gets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TailValue {
    Scalar(String),
    Sequence(Vec<String>),
}

/// Extracts an [`ExtractedRecord`] from `contents`. `path` is the record's
/// project-relative path: it is the sole source of the typed identity (see
/// [`extract_identity`]) and the filename-stem title fallback. Pure: no I/O.
pub fn extract_record(path: &Path, contents: &str) -> ExtractedRecord {
    let block = frontmatter_block(contents);
    let frontmatter = block.and_then(parse_frontmatter);
    let body = strip_frontmatter(contents).to_owned();

    let doc_type = frontmatter_scalar(block, "type").unwrap_or_default();
    let (number, concept_id, identity_kind) = extract_identity(path, &doc_type);
    let description = frontmatter_scalar(block, "description").unwrap_or_default();
    let title = frontmatter_scalar(block, "title")
        .or_else(|| first_heading(&body))
        .unwrap_or_else(|| filename_stem(path));
    let supersedes = frontmatter_scalar(block, "supersedes");
    let superseded_by = frontmatter_scalar(block, "superseded_by");
    let tags = frontmatter_sequence(frontmatter.as_ref(), "tags");
    let status = frontmatter_scalar(block, "status");
    let frontmatter_tail = extract_frontmatter_tail(frontmatter.as_ref());

    ExtractedRecord {
        doc_type,
        number,
        concept_id,
        identity_kind,
        title,
        description,
        body,
        supersedes,
        superseded_by,
        tags,
        status,
        frontmatter_tail,
    }
}

/// Classifies `doc_type` into `identity_kind` (ADR 0007 decision 2) and
/// derives exactly the matching identity field from `path`, leaving the
/// other one `None`. A `number:`/`concept_id:` frontmatter key is never
/// consulted (issue 0006 slice 0006-C1, lesson 3706): real OKF docs carry
/// no such field — the number is the filename's `NNNN` prefix and the
/// concept id is the path itself.
fn extract_identity(path: &Path, doc_type: &str) -> (Option<i32>, Option<String>, String) {
    if is_numbered_doc_type(doc_type) {
        (numbered_prefix(path), None, NUMBER_IDENTITY_KIND.to_owned())
    } else {
        (
            None,
            concept_id_from_path(path),
            CONCEPT_IDENTITY_KIND.to_owned(),
        )
    }
}

fn is_numbered_doc_type(doc_type: &str) -> bool {
    NUMBERED_DOC_TYPES.contains(&doc_type.to_lowercase().as_str())
}

/// The strict `NNNN-*.md` prefix of `path`'s filename (four ASCII digits
/// followed by `-`), mirroring
/// `living_docs_core::commands::next::numeric_prefix`. `None` when the
/// filename does not open with exactly four digits and a dash.
fn numbered_prefix(path: &Path) -> Option<i32> {
    let filename = path.file_name()?.to_str()?;
    if !filename.ends_with(".md") || filename.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let prefix = filename.get(0..4)?;
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse().ok()
}

/// `path` with a trailing `.md` removed, `None` only when `path` is not
/// valid UTF-8.
fn concept_id_from_path(path: &Path) -> Option<String> {
    path.to_str()?.strip_suffix(".md").map(str::to_owned)
}

/// Every frontmatter key with no typed column, in source encounter order
/// (ADR 0007 decision 1). Relies on `serde_yaml::Mapping` preserving the
/// document's key order. A key whose value is a YAML sequence carries a
/// [`TailValue::Sequence`] of its scalar elements (ADR 0019 slice S3b) — a
/// non-scalar sequence element is dropped, mirroring how a non-scalar,
/// non-sequence value is dropped entirely today. A key whose value is
/// neither a scalar nor a sequence (e.g. a nested mapping) is excluded,
/// exactly as before this extension.
fn extract_frontmatter_tail(frontmatter: Option<&Value>) -> Vec<(String, TailValue)> {
    let Some(mapping) = frontmatter.and_then(Value::as_mapping) else {
        return Vec::new();
    };

    mapping
        .iter()
        .filter_map(|(key, value)| {
            let key = key.as_str()?;
            if TYPED_FRONTMATTER_KEYS.contains(&key) {
                return None;
            }
            tail_value_from_yaml(value).map(|value| (key.to_owned(), value))
        })
        .collect()
}

/// Converts one frontmatter value into a [`TailValue`]: a scalar becomes
/// [`TailValue::Scalar`], a sequence becomes [`TailValue::Sequence`] of its
/// scalar elements (dropping any non-scalar element), and anything else
/// (e.g. a nested mapping) yields `None`.
fn tail_value_from_yaml(value: &Value) -> Option<TailValue> {
    if let Some(sequence) = value.as_sequence() {
        let items = sequence.iter().filter_map(scalar_to_string).collect();
        return Some(TailValue::Sequence(items));
    }
    scalar_to_string(value).map(TailValue::Scalar)
}

/// [`extract_record`]'s typed-scalar reads (`type`, `title`, `description`,
/// `supersedes`, `superseded_by`, `status`), delegating to the crate's
/// shared strict reader ([`read_scalar_strict`]) once `block` has already
/// been sliced.
fn frontmatter_scalar(block: Option<&str>, key: &str) -> Option<String> {
    read_scalar_strict(block?, key)
}

/// Reads `key` as a YAML sequence of scalars (the `tags: [a, b]` shape),
/// returning an empty vector when the key is absent or not a sequence.
fn frontmatter_sequence(frontmatter: Option<&Value>, key: &str) -> Vec<String> {
    let Some(mapping) = frontmatter.and_then(Value::as_mapping) else {
        return Vec::new();
    };
    let Some(sequence) = mapping
        .get(Value::String(key.to_owned()))
        .and_then(Value::as_sequence)
    else {
        return Vec::new();
    };
    sequence.iter().filter_map(scalar_to_string).collect()
}

fn strip_frontmatter(contents: &str) -> &str {
    let Some(rest) = contents.strip_prefix("---\n") else {
        return contents;
    };
    let Some(end) = rest.find("\n---") else {
        return contents;
    };
    let after_fence = &rest[end + 4..];
    after_fence.strip_prefix('\n').unwrap_or(after_fence)
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|line| line.strip_prefix("# ").map(|title| title.trim().to_owned()))
}

fn filename_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

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
/// output through [`extract_record`] reproduces every field `extract_record`
/// reads (ADR 0007 decision 3): the serializer's canonical order is a fixed
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

/// Renders one tail entry: a [`TailValue::Scalar`] as `key: value`, a
/// [`TailValue::Sequence`] as `key: [a, b]` — the same canonical flow style
/// [`push_tags`] already uses for `tags:`, quoting each element with
/// [`format_scalar`] (ADR 0019 slice S3b).
fn push_tail_value(lines: &mut Vec<String>, key: &str, value: &TailValue) {
    match value {
        TailValue::Scalar(scalar) => lines.push(format!("{key}: {}", format_scalar(scalar))),
        TailValue::Sequence(items) => {
            let rendered: Vec<String> = items.iter().map(|item| format_scalar(item)).collect();
            lines.push(format!("{key}: [{}]", rendered.join(", ")));
        }
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
    key == TRACKER_KEY || key == TIMESTAMP_KEY
}

const YAML_INDICATOR_PREFIXES: &str = "!&*-?|>%@`\"'#,[]{}";

/// Renders `value` as a plain YAML scalar when safe, or a double-quoted
/// scalar (with `\`/`"` escaped) when a plain scalar would be ambiguous —
/// empty, starting with a YAML indicator character, containing `": "`,
/// ending in `:`, or carrying leading/trailing whitespace.
pub(crate) fn format_scalar(value: &str) -> String {
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

    #[test]
    fn extract_record_derives_number_from_the_filenames_nnnn_prefix() {
        let contents = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\n---\n# 0001. Quokka Caching\n\nBody text.\n";
        let extracted = extract_record(Path::new("adr/0001-quokka-caching.md"), contents);

        assert_eq!(extracted.doc_type, "ADR");
        assert_eq!(extracted.number, Some(1));
        assert_eq!(extracted.concept_id, None);
        assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
        assert_eq!(extracted.title, "Quokka Caching");
        assert_eq!(extracted.description, "Adopt quokka caching.");
        assert_eq!(extracted.body, "# 0001. Quokka Caching\n\nBody text.\n");
    }

    #[test]
    fn extract_record_ignores_a_number_frontmatter_key_and_derives_from_the_path_instead() {
        let contents = "---\ntype: ADR\nnumber: 99\n---\nBody.\n";
        let extracted = extract_record(Path::new("adr/0007-x.md"), contents);

        assert_eq!(extracted.number, Some(7));
    }

    #[test]
    fn extract_record_derives_concept_id_from_the_project_relative_path() {
        let contents = "---\ntype: Glossary\ntitle: Findability\n---\n# Findability\n\nBody.\n";
        let extracted = extract_record(Path::new("context/user-auth.md"), contents);

        assert_eq!(extracted.identity_kind, CONCEPT_IDENTITY_KIND);
        assert_eq!(extracted.concept_id, Some("context/user-auth".to_owned()));
        assert_eq!(extracted.number, None);
    }

    #[test]
    fn extract_record_ignores_a_concept_id_frontmatter_key_and_derives_from_the_path_instead() {
        let contents = "---\ntype: Glossary\nconcept_id: wrong-slug\n---\nBody.\n";
        let extracted = extract_record(Path::new("glossary/findability.md"), contents);

        assert_eq!(
            extracted.concept_id,
            Some("glossary/findability".to_owned())
        );
    }

    #[test]
    fn extract_record_assigns_the_number_identity_kind_to_every_numbered_doc_type() {
        for doc_type in ["ADR", "BDR", "PRD", "Issue"] {
            let contents = format!("---\ntype: {doc_type}\n---\nBody.\n");
            let extracted = extract_record(Path::new("adr/0007-numbered.md"), &contents);

            assert_eq!(
                extracted.identity_kind, NUMBER_IDENTITY_KIND,
                "{doc_type} must classify as the number identity kind"
            );
            assert_eq!(extracted.number, Some(7));
            assert_eq!(extracted.concept_id, None);
        }
    }

    #[test]
    fn extract_record_yields_no_number_when_the_filename_lacks_a_valid_nnnn_prefix() {
        let contents = "---\ntype: ADR\n---\nBody.\n";
        let extracted = extract_record(Path::new("adr/no-number-here.md"), contents);

        assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
        assert_eq!(extracted.number, None);
    }

    #[test]
    fn extract_record_concept_doc_always_yields_a_concept_id_even_without_frontmatter() {
        let contents = "Body with no frontmatter block at all.\n";
        let extracted = extract_record(Path::new("glossary/findability.md"), contents);

        assert_eq!(extracted.identity_kind, CONCEPT_IDENTITY_KIND);
        assert_eq!(
            extracted.concept_id,
            Some("glossary/findability".to_owned())
        );
    }

    #[test]
    fn extract_record_falls_back_to_first_heading_when_title_is_absent() {
        let contents = "---\ntype: ADR\n---\n# Fallback Heading\n\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0002-fallback.md"), contents);

        assert_eq!(extracted.title, "Fallback Heading");
    }

    #[test]
    fn extract_record_falls_back_to_filename_stem_when_no_title_or_heading() {
        let contents = "---\ntype: ADR\n---\nBody with no heading.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0003-stemmed.md"), contents);

        assert_eq!(extracted.title, "0003-stemmed");
    }

    #[test]
    fn extract_record_defaults_missing_description_to_empty() {
        let contents = "---\ntype: ADR\ntitle: No Extras\n---\nBody.\n";
        let extracted = extract_record(Path::new("adr/0004-no-extras.md"), contents);

        assert_eq!(extracted.description, "");
        assert_eq!(extracted.concept_id, None);
    }

    #[test]
    fn extract_record_strips_the_frontmatter_block_from_the_body() {
        let contents = "---\ntype: ADR\ntitle: Stripped\n---\n# Stripped\n\nRemaining body.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0005-stripped.md"), contents);

        assert!(!extracted.body.contains("---"));
        assert_eq!(extracted.body, "# Stripped\n\nRemaining body.\n");
    }

    #[test]
    fn extract_record_without_frontmatter_treats_the_whole_file_as_body() {
        let contents = "# No Frontmatter\n\nJust a body.\n";
        let extracted = extract_record(Path::new("/bundle/log.md"), contents);

        assert_eq!(extracted.doc_type, "");
        assert_eq!(extracted.body, contents);
        assert_eq!(extracted.title, "No Frontmatter");
    }

    #[test]
    fn extract_record_ignores_a_concept_id_stray_on_a_numbered_doc_type() {
        let contents = "---\ntype: Issue\nconcept_id: findability\n---\nBody.\n";
        let extracted = extract_record(Path::new("issues/0006-findability.md"), contents);

        assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
        assert_eq!(extracted.number, Some(6));
        assert_eq!(
            extracted.concept_id, None,
            "concept_id is not the identity field for a numbered doc type"
        );
    }

    #[test]
    fn extract_record_tail_excludes_typed_and_relation_keys_in_source_order() {
        let contents = "---\ntype: ADR\ntitle: Tailed\ndescription: d.\nnumber: 1\nstatus: Accepted\nsupersedes:\nsuperseded_by:\ntags: [a]\ntracker: JIRA-1\ntimestamp: 2026-07-17T00:00:00Z\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0001-tailed.md"), contents);

        assert_eq!(
            extracted.frontmatter_tail,
            vec![
                ("tracker".to_owned(), TailValue::Scalar("JIRA-1".to_owned())),
                (
                    "timestamp".to_owned(),
                    TailValue::Scalar("2026-07-17T00:00:00Z".to_owned())
                ),
            ]
        );
        assert_eq!(extracted.status, Some("Accepted".to_owned()));
    }

    #[test]
    fn extract_record_reads_status_when_present() {
        let contents = "---\ntype: ADR\nstatus: Accepted\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0001-status.md"), contents);

        assert_eq!(extracted.status, Some("Accepted".to_owned()));
    }

    #[test]
    fn extract_record_defaults_missing_status_to_none() {
        let contents = "---\ntype: ADR\ntitle: No Status\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0002-no-status.md"), contents);

        assert_eq!(extracted.status, None);
    }

    #[test]
    fn extract_record_tail_is_empty_when_only_typed_keys_are_present() {
        let contents = "---\ntype: ADR\ntitle: No Tail\ndescription: d.\nnumber: 1\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0002-no-tail.md"), contents);

        assert!(extracted.frontmatter_tail.is_empty());
    }

    #[test]
    fn extract_record_reads_supersedes_superseded_by_and_tags() {
        let contents = "---\ntype: ADR\nsupersedes: 0001\nsuperseded_by: 0003\ntags: [caching, performance]\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0002-improved.md"), contents);

        assert_eq!(extracted.supersedes, Some("0001".to_owned()));
        assert_eq!(extracted.superseded_by, Some("0003".to_owned()));
        assert_eq!(
            extracted.tags,
            vec!["caching".to_owned(), "performance".to_owned()]
        );
    }

    #[test]
    fn extract_record_defaults_missing_supersede_fields_and_tags_to_empty() {
        let contents = "---\ntype: ADR\nsupersedes:\nsuperseded_by:\ntags: []\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0001-first.md"), contents);

        assert_eq!(extracted.supersedes, None);
        assert_eq!(extracted.superseded_by, None);
        assert!(extracted.tags.is_empty());
    }

    #[test]
    fn extract_record_defaults_tags_to_empty_when_the_key_is_absent() {
        let contents = "---\ntype: ADR\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0004-no-tags.md"), contents);

        assert!(extracted.tags.is_empty());
    }

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
                (
                    "labels".to_owned(),
                    TailValue::Scalar("important".to_owned()),
                ),
                (
                    "blocked_by".to_owned(),
                    TailValue::Scalar("0002".to_owned()),
                ),
                (
                    "tracker".to_owned(),
                    TailValue::Scalar("JIRA-42".to_owned()),
                ),
                (
                    "timestamp".to_owned(),
                    TailValue::Scalar("2026-07-17T00:00:00Z".to_owned()),
                ),
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

    /// An issue-style record whose tail carries list-valued `labels:` and
    /// `blocked_by:` keys (ADR 0019 slice S3b, closing the gap where
    /// `extract_frontmatter_tail` used to drop a sequence-valued key
    /// entirely).
    fn issue_record_with_list_valued_tail() -> ExtractedRecord {
        ExtractedRecord {
            doc_type: "Issue".to_owned(),
            number: Some(6),
            concept_id: None,
            identity_kind: NUMBER_IDENTITY_KIND.to_owned(),
            title: "Findability Search".to_owned(),
            description: "d.".to_owned(),
            body: "# 0006. Findability Search\n\nBody.\n".to_owned(),
            supersedes: None,
            superseded_by: None,
            tags: vec!["slice".to_owned()],
            status: Some("done".to_owned()),
            frontmatter_tail: vec![
                (
                    "labels".to_owned(),
                    TailValue::Sequence(vec![
                        "slice".to_owned(),
                        "skeleton".to_owned(),
                        "refactor".to_owned(),
                    ]),
                ),
                ("blocked_by".to_owned(), TailValue::Sequence(Vec::new())),
                (
                    "timestamp".to_owned(),
                    TailValue::Scalar("2026-07-16T00:00:00Z".to_owned()),
                ),
            ],
        }
    }

    #[test]
    fn to_canonical_markdown_emits_a_sequence_tail_value_in_flow_style() {
        let record = issue_record_with_list_valued_tail();

        let markdown = to_canonical_markdown(&record);

        assert!(markdown.contains("labels: [slice, skeleton, refactor]"));
        assert!(markdown.contains("blocked_by: []"));
    }

    #[test]
    fn to_canonical_markdown_round_trips_an_issue_record_with_list_valued_tail_keys() {
        let record = issue_record_with_list_valued_tail();

        let markdown = to_canonical_markdown(&record);
        let reparsed = extract_record(Path::new("issues/0006-findability-search.md"), &markdown);

        assert_round_trips(&reparsed, &record);
        assert_eq!(
            reparsed.frontmatter_tail, record.frontmatter_tail,
            "labels/blocked_by must round-trip as the same ordered list of scalars"
        );
    }

    #[test]
    fn extract_record_reads_a_list_valued_tail_key_as_a_sequence() {
        let contents = "---\ntype: Issue\ntitle: T\ndescription: d.\nlabels: [slice, skeleton]\nblocked_by: []\n---\nBody.\n";
        let extracted = extract_record(Path::new("issues/0001-t.md"), contents);

        assert_eq!(
            extracted.frontmatter_tail,
            vec![
                (
                    "labels".to_owned(),
                    TailValue::Sequence(vec!["slice".to_owned(), "skeleton".to_owned()])
                ),
                ("blocked_by".to_owned(), TailValue::Sequence(Vec::new())),
            ]
        );
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
