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
/// column (plus a `supersedes` list that is not all record numbers and a
/// `tags` list holding a number or boolean item, which keep their item
/// types), in source encounter order, ready to insert into
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
    /// Every record this one replaces, in written order: one entry for a
    /// scalar `supersedes: NNNN`, several for a `supersedes: [NNNN, NNNN]`
    /// list, empty for `supersedes: []`, `None` when the key is absent or
    /// holds no readable value. Entries are kept exactly as written.
    pub supersedes: Option<Vec<String>>,
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
/// quoting each element by the `needs_quoting` rules and also any element
/// whose plain form, read back inside a flow sequence, would give another
/// value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TailValue {
    Scalar(String),
    Sequence(Vec<String>),
    /// A sequence with a number, boolean, null or empty-string item, kept as
    /// parsed so each item is written back with its YAML type.
    Typed(Vec<Value>),
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
    let supersedes = frontmatter_scalar_or_list(frontmatter.as_ref(), "supersedes");
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

/// [`extract_record`]'s typed-scalar reads (`type`, `title`, `description`,
/// `superseded_by`, `status`, `owner`), delegating to the crate's
/// shared strict reader ([`read_scalar_strict`]) once `block` has already
/// been sliced.
fn frontmatter_scalar(block: Option<&str>, key: &str) -> Option<String> {
    read_scalar_strict(block?, key)
}

/// Reads `key` as a single scalar (one entry) or a sequence of scalars (its
/// entries in order). `None` when the key is absent, null, a mapping, or a
/// sequence holding an entry that is not a record number, which the
/// frontmatter tail carries as a typed sequence instead.
fn frontmatter_scalar_or_list(frontmatter: Option<&Value>, key: &str) -> Option<Vec<String>> {
    let value = frontmatter?
        .as_mapping()?
        .get(Value::String(key.to_owned()))?;
    match value.as_sequence() {
        Some(sequence) => record_numbers(sequence),
        None => scalar_to_string(value).map(|scalar| vec![scalar]),
    }
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

/// The text of the first level-1 heading outside a fenced code block, with a
/// leading `NNNN. ` number removed.
fn first_heading(body: &str) -> Option<String> {
    let mut fenced = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
        } else if !fenced {
            if let Some(title) = line.strip_prefix("# ") {
                return Some(strip_number(title.trim(), ". ").to_owned());
            }
        }
    }
    None
}

/// The filename stem with a leading `NNNN-` removed.
fn filename_stem(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    strip_number(stem, "-").to_owned()
}

/// `text` without a leading four-digit number followed by `separator`;
/// unchanged when that prefix is absent.
fn strip_number<'a>(text: &'a str, separator: &str) -> &'a str {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits != 4 {
        return text;
    }
    text[4..].strip_prefix(separator).unwrap_or(text)
}

mod canonical;
use canonical::extract_frontmatter_tail;
pub use canonical::to_canonical_markdown;
pub(crate) use canonical::{format_flow_list, format_scalar, kept_tag, record_numbers};

#[cfg(test)]
mod tests;
