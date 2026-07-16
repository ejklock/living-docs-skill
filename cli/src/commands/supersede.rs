use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const RECORD_DIRS: [&str; 4] = ["adr", "bdr", "prd", "issues"];

pub fn run(docs_dir: &Path, old: &str, new: &str) -> ExitCode {
    match supersede(docs_dir, old, new) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs supersede: {message}");
            ExitCode::from(2)
        }
    }
}

fn supersede(docs_dir: &Path, old: &str, new: &str) -> Result<(), String> {
    let old_number = parse_record_number(old)?;
    let new_number = parse_record_number(new)?;

    let old_path = find_record(docs_dir, old_number)?;
    let new_path = find_record(docs_dir, new_number)?;

    set_frontmatter_field(&old_path, "status", "Superseded")?;
    set_frontmatter_field(&old_path, "superseded_by", &format!("{new_number:04}"))?;
    set_frontmatter_field(&new_path, "supersedes", &format!("{old_number:04}"))?;

    Ok(())
}

fn parse_record_number(arg: &str) -> Result<u32, String> {
    arg.parse()
        .map_err(|_| format!("'{arg}' is not a valid record number"))
}

fn find_record(docs_dir: &Path, number: u32) -> Result<PathBuf, String> {
    let prefix = format!("{number:04}-");

    RECORD_DIRS
        .iter()
        .find_map(|dir_name| find_in_dir(&docs_dir.join(dir_name), &prefix))
        .ok_or_else(|| format!("no record found for {number:04}"))
}

fn find_in_dir(type_dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(type_dir).ok()?;
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".md"))
        })
}

/// Sets `key`'s value inside the leading frontmatter block via a targeted line
/// edit — reusing S2's approach (`new.rs`'s `replace_targeted_value`) rather than a
/// serde round-trip, so comments and the body survive untouched. Templates ship
/// most supersede keys as an empty line to fill; when a key is absent entirely
/// (e.g. BDR/PRD templates have no `supersedes` line), it is inserted at the end
/// of the frontmatter block instead.
fn set_frontmatter_field(path: &Path, key: &str, value: &str) -> Result<(), String> {
    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let updated = apply_frontmatter_field(&contents, key, value)
        .ok_or_else(|| format!("{}: missing frontmatter block", path.display()))?;
    fs::write(path, updated).map_err(|e| e.to_string())
}

fn apply_frontmatter_field(contents: &str, key: &str, value: &str) -> Option<String> {
    let lines: Vec<&str> = contents.lines().collect();
    let close = frontmatter_close_index(&lines)?;
    let prefix = format!("{key}:");

    let mut updated: Vec<String> = lines.iter().map(|&line| line.to_string()).collect();
    match lines[1..close]
        .iter()
        .position(|&line| line.starts_with(&prefix))
    {
        Some(relative_index) => {
            let index = relative_index + 1;
            updated[index] = set_targeted_value(lines[index], &prefix, value);
        }
        None => updated.insert(close, format!("{prefix} {value}")),
    }

    Some(updated.join("\n") + "\n")
}

fn frontmatter_close_index(lines: &[&str]) -> Option<usize> {
    if lines.first() != Some(&"---") {
        return None;
    }
    lines
        .iter()
        .skip(1)
        .position(|&line| line == "---")
        .map(|i| i + 1)
}

/// Replaces the value of a `key: value` frontmatter line, preserving any
/// trailing `# guidance comment` verbatim — mirrors `new.rs::replace_targeted_value`.
fn set_targeted_value(line: &str, prefix: &str, new_value: &str) -> String {
    let rest = line.strip_prefix(prefix).unwrap_or_default();
    match rest.find('#') {
        Some(hash_idx) => format!("{prefix} {new_value} {}", &rest[hash_idx..]),
        None => format!("{prefix} {new_value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_frontmatter_field_fills_an_existing_empty_key_line() {
        let contents = "---\ntype: ADR\nsupersedes:\nsuperseded_by:\n---\n\n# Body\n";
        let updated = apply_frontmatter_field(contents, "superseded_by", "0002").unwrap();
        assert!(updated.contains("superseded_by: 0002"));
        assert!(updated.contains("supersedes:\n"));
    }

    #[test]
    fn apply_frontmatter_field_preserves_a_trailing_guidance_comment() {
        let contents = "---\nsupersedes:                 # NNNN of the ADR this replaces, if any\n---\n\n# Body\n";
        let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
        assert!(updated.contains("supersedes: 0001 # NNNN of the ADR this replaces, if any"));
    }

    #[test]
    fn apply_frontmatter_field_inserts_an_absent_key_before_the_closing_fence() {
        let contents = "---\ntype: BDR\nsuperseded_by:\n---\n\n# Body\n";
        let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
        assert!(updated.contains("supersedes: 0001"));
        assert!(updated.contains("---\ntype: BDR\nsuperseded_by:\nsupersedes: 0001\n---"));
    }

    #[test]
    fn apply_frontmatter_field_leaves_the_body_untouched() {
        let contents = "---\ntype: ADR\nsupersedes:\n---\n\n## Context\n\nSome body text.\n";
        let updated = apply_frontmatter_field(contents, "supersedes", "0001").unwrap();
        assert!(updated.contains("## Context\n\nSome body text.\n"));
    }

    #[test]
    fn apply_frontmatter_field_without_a_frontmatter_block_returns_none() {
        assert_eq!(
            apply_frontmatter_field("no frontmatter here\n", "supersedes", "0001"),
            None
        );
    }

    #[test]
    fn parse_record_number_rejects_non_numeric_input() {
        assert!(parse_record_number("abcd").is_err());
    }
}
