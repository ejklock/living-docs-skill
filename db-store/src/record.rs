//! Pure frontmatter/body extraction feeding [`crate::sync::sync_project`]
//! (ADR 0004, issue 0002 slice S2b; supersedes/superseded_by/tags parsing
//! ADR 0005 issue 0005 slice 0005-B; dual typed identity + EAV frontmatter
//! tail ADR 0007 issue 0006 slice 0006-A). Every function here takes
//! already-read file contents; none touches the filesystem.
//!
//! [`ExtractedRecord`] and the [`NUMBER_IDENTITY_KIND`]/
//! [`CONCEPT_IDENTITY_KIND`] constants are shared with
//! [`crate::serialize::to_canonical_markdown`] (issue 0006 slice 0006-B),
//! which is this module's inverse: whatever [`extract_record`] parses out
//! of a `.md` file, `to_canonical_markdown` reconstructs from an
//! `ExtractedRecord` back into one.

use std::path::Path;

use serde_yaml::Value;

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
const TYPED_FRONTMATTER_KEYS: [&str; 8] = [
    "type",
    "title",
    "description",
    "number",
    "concept_id",
    "supersedes",
    "superseded_by",
    "tags",
];

/// A single ranked full-text search hit: the record's bundle-relative path,
/// its title, an FTS5 snippet highlighting the matched term, and the slug of
/// the project it belongs to (ADR 0005, issue 0005 slice 0005-C1). Every
/// hit carries `project` regardless of whether the search that produced it
/// was scoped to one project or spanned all of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub project: String,
}

/// The fields extracted from a doc record's raw contents, ready to insert
/// into the `records` table. `identity_kind` is derived from `doc_type`
/// (ADR 0007 decision 2): a numbered type (adr/bdr/prd/issue) carries
/// `number` with `concept_id` left `None`, every other type carries
/// `concept_id` with `number` left `None`. `supersedes`/`superseded_by`
/// carry the raw `NNNN` frontmatter value (unresolved to a record id — that
/// resolution happens against a project's other records in
/// [`crate::sync::sync_project`]); `tags` is the frontmatter's `tags`
/// sequence, empty when absent. `frontmatter_tail` is every remaining
/// frontmatter key with no typed column, in source encounter order, ready
/// to insert into `frontmatter_fields` with the index as `ordinal`.
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
    pub frontmatter_tail: Vec<(String, String)>,
}

/// True for the two reserved filenames that carry no OKF frontmatter and are
/// excluded from the read-model, mirroring
/// `living_docs_core::check::records::is_reserved`.
pub fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("index.md") | Some("log.md")
    )
}

/// Extracts an [`ExtractedRecord`] from `contents`. `path` is used only for
/// the filename-stem title fallback. Pure: no I/O.
pub fn extract_record(path: &Path, contents: &str) -> ExtractedRecord {
    let frontmatter = frontmatter_block(contents).and_then(parse_frontmatter);
    let body = strip_frontmatter(contents).to_owned();

    let doc_type = frontmatter_scalar(frontmatter.as_ref(), "type").unwrap_or_default();
    let (number, concept_id, identity_kind) = extract_identity(frontmatter.as_ref(), &doc_type);
    let description = frontmatter_scalar(frontmatter.as_ref(), "description").unwrap_or_default();
    let title = frontmatter_scalar(frontmatter.as_ref(), "title")
        .or_else(|| first_heading(&body))
        .unwrap_or_else(|| filename_stem(path));
    let supersedes = frontmatter_scalar(frontmatter.as_ref(), "supersedes");
    let superseded_by = frontmatter_scalar(frontmatter.as_ref(), "superseded_by");
    let tags = frontmatter_sequence(frontmatter.as_ref(), "tags");
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
        frontmatter_tail,
    }
}

/// Classifies `doc_type` into `identity_kind` (ADR 0007 decision 2) and
/// reads exactly the matching identity field from `frontmatter`, leaving
/// the other one `None`.
fn extract_identity(
    frontmatter: Option<&Value>,
    doc_type: &str,
) -> (Option<i32>, Option<String>, String) {
    if is_numbered_doc_type(doc_type) {
        let number = frontmatter_scalar(frontmatter, "number").and_then(|raw| raw.parse().ok());
        (number, None, NUMBER_IDENTITY_KIND.to_owned())
    } else {
        let concept_id = frontmatter_scalar(frontmatter, "concept_id");
        (None, concept_id, CONCEPT_IDENTITY_KIND.to_owned())
    }
}

fn is_numbered_doc_type(doc_type: &str) -> bool {
    NUMBERED_DOC_TYPES.contains(&doc_type.to_lowercase().as_str())
}

/// Every frontmatter key with no typed column, in source encounter order
/// (ADR 0007 decision 1). Relies on `serde_yaml::Mapping` preserving the
/// document's key order.
fn extract_frontmatter_tail(frontmatter: Option<&Value>) -> Vec<(String, String)> {
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
            scalar_to_string(value).map(|value| (key.to_owned(), value))
        })
        .collect()
}

fn frontmatter_block(contents: &str) -> Option<&str> {
    let rest = contents.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn parse_frontmatter(block: &str) -> Option<Value> {
    serde_yaml::from_str(block).ok()
}

fn frontmatter_scalar(frontmatter: Option<&Value>, key: &str) -> Option<String> {
    let mapping = frontmatter?.as_mapping()?;
    let value = mapping.get(Value::String(key.to_owned()))?;
    scalar_to_string(value)
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

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn is_reserved_matches_index_and_log_only() {
        assert!(is_reserved(Path::new("/bundle/index.md")));
        assert!(is_reserved(Path::new("/bundle/log.md")));
        assert!(!is_reserved(Path::new("/bundle/adr/0001-title.md")));
    }

    #[test]
    fn extract_record_reads_frontmatter_title_description_and_identity() {
        let contents = "---\ntype: ADR\ntitle: Quokka Caching\ndescription: Adopt quokka caching.\nnumber: 1\n---\n# 0001. Quokka Caching\n\nBody text.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0001-quokka-caching.md"), contents);

        assert_eq!(extracted.doc_type, "ADR");
        assert_eq!(extracted.number, Some(1));
        assert_eq!(extracted.concept_id, None);
        assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
        assert_eq!(extracted.title, "Quokka Caching");
        assert_eq!(extracted.description, "Adopt quokka caching.");
        assert_eq!(extracted.body, "# 0001. Quokka Caching\n\nBody text.\n");
    }

    #[test]
    fn extract_record_assigns_the_concept_identity_kind_to_a_non_numbered_doc_type() {
        let contents =
            "---\ntype: Glossary\nconcept_id: findability\n---\n# Findability\n\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/glossary/findability.md"), contents);

        assert_eq!(extracted.identity_kind, CONCEPT_IDENTITY_KIND);
        assert_eq!(extracted.concept_id, Some("findability".to_owned()));
        assert_eq!(extracted.number, None);
    }

    #[test]
    fn extract_record_assigns_the_number_identity_kind_to_every_numbered_doc_type() {
        for doc_type in ["ADR", "BDR", "PRD", "Issue"] {
            let contents = format!("---\ntype: {doc_type}\nnumber: 7\n---\nBody.\n");
            let extracted = extract_record(Path::new("/bundle/adr/0007-numbered.md"), &contents);

            assert_eq!(
                extracted.identity_kind, NUMBER_IDENTITY_KIND,
                "{doc_type} must classify as the number identity kind"
            );
            assert_eq!(extracted.number, Some(7));
            assert_eq!(extracted.concept_id, None);
        }
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
    fn extract_record_defaults_missing_description_and_identity_to_empty() {
        let contents = "---\ntype: ADR\ntitle: No Extras\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/adr/0004-no-extras.md"), contents);

        assert_eq!(extracted.description, "");
        assert_eq!(extracted.number, None);
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
        let extracted = extract_record(Path::new("/bundle/issues/0006-findability.md"), contents);

        assert_eq!(extracted.identity_kind, NUMBER_IDENTITY_KIND);
        assert_eq!(extracted.number, None);
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
                ("status".to_owned(), "Accepted".to_owned()),
                ("tracker".to_owned(), "JIRA-1".to_owned()),
                ("timestamp".to_owned(), "2026-07-17T00:00:00Z".to_owned()),
            ]
        );
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

    #[test]
    fn search_hit_carries_path_title_snippet_and_project() {
        let hit = SearchHit {
            path: "adr/0001-quokka-caching.md".to_owned(),
            title: "Quokka Caching".to_owned(),
            snippet: "an aggressive [quokka] caching strategy".to_owned(),
            project: "team-a".to_owned(),
        };

        assert_eq!(
            hit.path,
            PathBuf::from("adr/0001-quokka-caching.md").to_string_lossy()
        );
        assert!(hit.snippet.contains("[quokka]"));
        assert_eq!(hit.project, "team-a");
    }
}
