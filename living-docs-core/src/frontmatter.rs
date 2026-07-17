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
pub fn read_scalar_from_str(contents: &str, key: &str) -> Option<String> {
    let block = extract_frontmatter_block(contents)?;
    let document: Value = serde_yaml::from_str(block).ok()?;
    let mapping = document.as_mapping()?;
    let value = mapping.get(Value::String(key.to_string()))?;
    scalar_to_string(value)
}

fn extract_frontmatter_block(contents: &str) -> Option<&str> {
    let rest = contents.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
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
}
