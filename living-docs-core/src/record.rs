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

use crate::doc_type::{self, Identity};
use crate::frontmatter::{
    frontmatter_block, parse_frontmatter, read_scalar_strict, scalar_to_string,
};

/// The `identity_kind` discriminator for a sequentially numbered doc
/// (`NNNN`, e.g. adr/prd/issue).
pub const NUMBER_IDENTITY_KIND: &str = "number";

/// The `identity_kind` discriminator for a path-identified OKF concept doc.
pub const CONCEPT_IDENTITY_KIND: &str = "concept";

/// Frontmatter keys that already have a universal typed column or dedicated
/// handling elsewhere, and therefore never land in the EAV
/// [`ExtractedRecord::frontmatter_tail`] (ADR 0007 decision 1).
const TYPED_FRONTMATTER_KEYS: [&str; 10] = [
    "type",
    "title",
    "description",
    "number",
    "concept_id",
    "supersedes",
    "superseded_by",
    "tags",
    "status",
    "owner",
];

/// The fields extracted from a doc record's raw contents, ready to insert
/// into the `records` table. `identity_kind` is derived from `doc_type`
/// (ADR 0007 decision 2): a numbered type (adr/prd/issue) carries
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
/// value, `None` when the key is absent (issue 0045, ADR 0015, S1).
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
    /// The record's accountable owner: a free-form name or email, `None`
    /// when the frontmatter carries no `owner:` value. The tool never
    /// validates this against any identity directory.
    pub owner: Option<String>,
    pub frontmatter_tail: Vec<(String, TailValue)>,
}

/// A [`ExtractedRecord::frontmatter_tail`] entry's value: a scalar `key: value`,
/// or the ordered sequence a `key: [a, b]` flow list parses into.
/// [`to_canonical_markdown`] re-emits a `Sequence` in the flow style tags use,
/// quoting each element with the [`format_scalar`] rules a `Scalar` gets.
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
    let owner = frontmatter_scalar(block, "owner");
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
        owner,
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

/// True when `doc_type` (matched case-insensitively) carries a sequential
/// `NNNN` identity rather than a `concept_id` (ADR 0007 decision 2), derived
/// from the doc-type registry's [`Identity::Numbered`] variant rather than a
/// hand-maintained list (ADR 0026).
fn is_numbered_doc_type(doc_type: &str) -> bool {
    matches!(
        doc_type::spec_for(&doc_type.to_lowercase()).map(|spec| spec.identity),
        Some(Identity::Numbered { .. })
    )
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
/// block in the fixed field order — `type`, `title`, `description`, `owner`
/// (if present), `status` (if present), `supersedes`, `superseded_by`,
/// `tags`, the remaining frontmatter tail by ascending ordinal, `tracker`
/// (if present), then `timestamp` (if present) — followed by a blank line
/// and the body. The typed identity (`number`/`concept_id`) is never
/// emitted: it is carried by the record's path, not its frontmatter.
/// Re-parsing the output through [`extract_record`] reproduces every field
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
    lines.push(format!("tags: {}", format_flow_list(&sorted)));
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
        TailValue::Sequence(items) => lines.push(format!("{key}: {}", format_flow_list(items))),
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

/// Renders `items` as a flow sequence (`[a, b]`), each quoted by [`format_scalar`].
pub(crate) fn format_flow_list<S: AsRef<str>>(items: &[S]) -> String {
    let items: Vec<String> = items.iter().map(|i| format_scalar(i.as_ref())).collect();
    format!("[{}]", items.join(", "))
}

const YAML_INDICATOR_PREFIXES: &str = "!&*-?|>%@`\"'#,[]{}";

/// Renders `value` as a plain YAML scalar when safe, or a double-quoted
/// scalar (with `\`/`"` escaped) when a plain scalar would be ambiguous —
/// empty, starting with a YAML indicator character, containing `": "` or a
/// ` #` comment opener, ending in `:`, or carrying surrounding whitespace.
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
        || value.contains(" #")
        || value.contains("\t#")
        || value.trim() != value
}

#[cfg(test)]
mod tests;
