use serde_yaml::Value;
use std::fs;
use std::path::Path;

/// Reads a top-level scalar from the leading `---`-fenced YAML frontmatter block of `path`.
///
/// Only top-level keys are considered — a key nested under another mapping does not
/// satisfy the lookup, even if the names match (the "05-nested-key-trap" fixture).
pub fn read_scalar(path: &Path, key: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    read_scalar_from_str(&contents, key)
}

/// [`read_scalar`]'s pure counterpart: reads a top-level scalar from
/// already-read `contents` instead of a filesystem path, so a caller sourcing
/// content through a [`crate::store::DocStore`] port (which may have no
/// backing file at all) can parse it without a second frontmatter reader.
///
/// Tries the canonical `serde_yaml` mapping parse first. A plain YAML scalar
/// whose value itself contains a bare `: ` is syntactically invalid YAML
/// (the parser reads it as an unexpected nested mapping) and fails the whole
/// document parse — this is a real, unquoted authoring shape (issue 0021
/// cause 3), so [`raw_scalar_line`] recovers `key`'s value straight from its
/// physical line whenever the mapping parse can't produce it.
pub fn read_scalar_from_str(contents: &str, key: &str) -> Option<String> {
    let block = extract_frontmatter_block(contents)?;
    mapping_scalar(block, key).or_else(|| raw_scalar_line(block, key))
}

fn extract_frontmatter_block(contents: &str) -> Option<&str> {
    let rest = contents.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn mapping_scalar(block: &str, key: &str) -> Option<String> {
    let document: Value = serde_yaml::from_str(block).ok()?;
    let mapping = document.as_mapping()?;
    let value = mapping.get(Value::String(key.to_string()))?;
    scalar_to_string(value)
}

/// A top-level (unindented) `key: value` line's value, taken verbatim rather
/// than YAML-parsed, so a value containing a bare `: ` (invalid as a plain
/// YAML scalar) is still recovered. A blank value or a matching pair of
/// surrounding quotes is handled the same way [`scalar_to_string`] and YAML
/// itself would.
fn raw_scalar_line(block: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let line = block.lines().find(|line| line.starts_with(&prefix))?;
    let raw_value = line[prefix.len()..].trim();
    if raw_value.is_empty() {
        return None;
    }
    Some(unquote(raw_value))
}

fn unquote(raw: &str) -> String {
    if let Some(inner) = strip_matching_quotes(raw, '"') {
        return inner.replace("\\\"", "\"").replace("\\\\", "\\");
    }
    if let Some(inner) = strip_matching_quotes(raw, '\'') {
        return inner.replace("''", "'");
    }
    raw.to_string()
}

fn strip_matching_quotes(raw: &str, quote: char) -> Option<&str> {
    let inner = raw.strip_prefix(quote)?.strip_suffix(quote)?;
    Some(inner)
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_doc(contents: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("living-docs-fm-test-{nanos}.md"));
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn reads_a_top_level_scalar() {
        let path = write_temp_doc("---\ntype: ADR\nstatus: Proposed\n---\n# Body\n");
        assert_eq!(read_scalar(&path, "type"), Some("ADR".to_string()));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn nested_key_does_not_rescue_a_missing_top_level_key() {
        let path = write_temp_doc("---\nmeta:\n  type: Reference\n---\n# Foo\n");
        assert_eq!(read_scalar(&path, "type"), None);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_frontmatter_block_returns_none() {
        let path = write_temp_doc("# No frontmatter here\n");
        assert_eq!(read_scalar(&path, "type"), None);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn empty_scalar_value_returns_none() {
        let path = write_temp_doc("---\ntype:\n---\n# Body\n");
        assert_eq!(read_scalar(&path, "type"), None);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_scalar_and_read_scalar_from_str_agree_on_the_same_document() {
        let contents = "---\ntype: ADR\nstatus: Proposed\n---\n# Body\n";
        let path = write_temp_doc(contents);

        assert_eq!(
            read_scalar(&path, "status"),
            read_scalar_from_str(contents, "status")
        );
        assert_eq!(read_scalar(&path, "status"), Some("Proposed".to_string()));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn read_scalar_and_read_scalar_from_str_agree_on_an_apostrophe_and_semicolon_title() {
        let contents = "---\ntype: ADR\ntitle: Provenance audit vs Matt Pocock's skills; remove improve-codebase-architecture as a vendored derivative\nstatus: Accepted\n---\n# Body\n";
        let path = write_temp_doc(contents);
        let expected = Some(
            "Provenance audit vs Matt Pocock's skills; remove improve-codebase-architecture as a vendored derivative"
                .to_string(),
        );

        assert_eq!(
            read_scalar(&path, "title"),
            read_scalar_from_str(contents, "title")
        );
        assert_eq!(read_scalar(&path, "title"), expected);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn reads_an_unquoted_title_whose_value_itself_contains_a_colon() {
        let contents = "---\ntype: ADR\ntitle: Provenance audit: vs Matt Pocock's skills; remove derivative\nstatus: Accepted\n---\n# Body\n";

        assert_eq!(
            read_scalar_from_str(contents, "title"),
            Some("Provenance audit: vs Matt Pocock's skills; remove derivative".to_string())
        );
        assert_eq!(
            read_scalar_from_str(contents, "status"),
            Some("Accepted".to_string())
        );
    }

    #[test]
    fn a_nested_key_still_does_not_rescue_a_missing_top_level_key_when_another_value_breaks_yaml_parsing(
    ) {
        let contents = "---\ntitle: Broken: value\nmeta:\n  type: Reference\n---\n# Body\n";

        assert_eq!(read_scalar_from_str(contents, "type"), None);
    }
}
