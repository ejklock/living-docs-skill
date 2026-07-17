//! Pure frontmatter/body extraction feeding [`crate::sync::sync`] (ADR 0004,
//! issue 0002 slice S2b). Every function here takes already-read file
//! contents; none touches the filesystem.

use std::path::Path;

use serde_yaml::Value;

/// A single ranked full-text search hit: the record's bundle-relative path,
/// its title, and an FTS5 snippet highlighting the matched term.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: String,
    pub title: String,
    pub snippet: String,
}

/// The fields extracted from a doc record's raw contents, ready to insert
/// into the `records` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractedRecord {
    pub doc_type: String,
    pub identity: Option<String>,
    pub title: String,
    pub description: String,
    pub body: String,
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
    let identity = frontmatter_scalar(frontmatter.as_ref(), "number")
        .or_else(|| frontmatter_scalar(frontmatter.as_ref(), "concept_id"));
    let description = frontmatter_scalar(frontmatter.as_ref(), "description").unwrap_or_default();
    let title = frontmatter_scalar(frontmatter.as_ref(), "title")
        .or_else(|| first_heading(&body))
        .unwrap_or_else(|| filename_stem(path));

    ExtractedRecord {
        doc_type,
        identity,
        title,
        description,
        body,
    }
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
        assert_eq!(extracted.identity, Some("1".to_owned()));
        assert_eq!(extracted.title, "Quokka Caching");
        assert_eq!(extracted.description, "Adopt quokka caching.");
        assert_eq!(extracted.body, "# 0001. Quokka Caching\n\nBody text.\n");
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
        assert_eq!(extracted.identity, None);
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
    fn extract_record_prefers_concept_id_over_missing_number() {
        let contents = "---\ntype: Issue\nconcept_id: findability\n---\nBody.\n";
        let extracted = extract_record(Path::new("/bundle/issues/0006-findability.md"), contents);

        assert_eq!(extracted.identity, Some("findability".to_owned()));
    }

    #[test]
    fn search_hit_carries_path_title_and_snippet() {
        let hit = SearchHit {
            path: "adr/0001-quokka-caching.md".to_owned(),
            title: "Quokka Caching".to_owned(),
            snippet: "an aggressive [quokka] caching strategy".to_owned(),
        };

        assert_eq!(
            hit.path,
            PathBuf::from("adr/0001-quokka-caching.md").to_string_lossy()
        );
        assert!(hit.snippet.contains("[quokka]"));
    }
}
