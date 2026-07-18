use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, docs_dir: &Path, old: &str, new: &str) -> ExitCode {
    match supersede(store, docs_dir, old, new) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs supersede: {message}");
            ExitCode::from(2)
        }
    }
}

fn supersede(store: &dyn DocStore, docs_dir: &Path, old: &str, new: &str) -> Result<(), String> {
    let old_number = parse_record_number(old)?;
    let new_number = parse_record_number(new)?;

    let old_path = find_record(store, docs_dir, old_number)?;
    let new_path = find_record(store, docs_dir, new_number)?;

    set_frontmatter_fields(
        store,
        &old_path,
        &[
            ("status", "Superseded".to_string()),
            ("superseded_by", format!("{new_number:04}")),
        ],
    )?;
    set_frontmatter_fields(
        store,
        &new_path,
        &[("supersedes", format!("{old_number:04}"))],
    )?;

    Ok(())
}

fn parse_record_number(arg: &str) -> Result<u32, String> {
    arg.parse()
        .map_err(|_| format!("'{arg}' is not a valid record number"))
}

/// Finds the record whose filename opens with `number`'s zero-padded `NNNN-`
/// prefix among every path the active store lists under `docs_dir` — backend
/// agnostic, so a db-mode store's own project-scoped enumeration is honored
/// exactly like a filesystem walk.
fn find_record(store: &dyn DocStore, docs_dir: &Path, number: u32) -> Result<PathBuf, String> {
    let prefix = format!("{number:04}-");
    store
        .list(docs_dir)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|path| matches_record_prefix(path, &prefix))
        .ok_or_else(|| format!("no record found for {number:04}"))
}

fn matches_record_prefix(path: &Path, prefix: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".md"))
}

/// Reads the record at `path` once, applies every `(key, value)` pair to its
/// leading frontmatter block in order via [`apply_frontmatter_field`] — a
/// targeted line edit (reusing S2's approach, `new.rs`'s
/// `replace_targeted_value`) rather than a serde round-trip, so comments and
/// the body survive untouched — then writes the result back once. Templates
/// ship most supersede keys as an empty line to fill; when a key is absent
/// entirely (e.g. BDR/PRD templates have no `supersedes` line), it is
/// inserted at the end of the frontmatter block instead.
fn set_frontmatter_fields(
    store: &dyn DocStore,
    path: &Path,
    fields: &[(&str, String)],
) -> Result<(), String> {
    let contents = store.read(path).map_err(|e| e.to_string())?;
    let updated = fields
        .iter()
        .try_fold(contents, |acc, (key, value)| {
            apply_frontmatter_field(&acc, key, value)
        })
        .ok_or_else(|| format!("{}: missing frontmatter block", path.display()))?;
    store.write(path, &updated).map_err(|e| e.to_string())
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

    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;

    /// A minimal in-memory [`DocStore`] test double, so `supersede`'s tests
    /// need no filesystem at all — the port read-modify-write is exercised
    /// directly against the store rather than through a temp directory.
    struct MapStore {
        files: RefCell<BTreeMap<PathBuf, String>>,
    }

    impl MapStore {
        fn seeded(seed: &[(&str, &str)]) -> Self {
            let files = seed
                .iter()
                .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
                .collect();
            Self {
                files: RefCell::new(files),
            }
        }
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .borrow()
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .borrow()
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
            self.files
                .borrow_mut()
                .insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }
    }

    const OLD_RECORD: &str =
        "---\ntype: ADR\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# Old\n";
    const NEW_RECORD: &str =
        "---\ntype: ADR\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# New\n";

    #[test]
    fn supersede_persists_status_and_both_links_through_the_store_read_modify_write() {
        let store = MapStore::seeded(&[
            ("/bundle/adr/0001-old.md", OLD_RECORD),
            ("/bundle/adr/0002-new.md", NEW_RECORD),
        ]);

        supersede(&store, Path::new("/bundle"), "0001", "0002").expect("supersede should succeed");

        let old = store
            .read(Path::new("/bundle/adr/0001-old.md"))
            .expect("old record still present");
        let new = store
            .read(Path::new("/bundle/adr/0002-new.md"))
            .expect("new record still present");

        assert!(old.contains("status: Superseded"), "got: {old}");
        assert!(old.contains("superseded_by: 0002"), "got: {old}");
        assert!(new.contains("supersedes: 0001"), "got: {new}");
    }

    #[test]
    fn supersede_fails_when_the_store_lists_no_record_for_a_number() {
        let store = MapStore::seeded(&[("/bundle/adr/0001-old.md", OLD_RECORD)]);

        let err = supersede(&store, Path::new("/bundle"), "0001", "0099")
            .expect_err("supersede must fail when the new record cannot be found");

        assert!(err.contains("no record found for 0099"), "got: {err}");
    }

    #[test]
    fn find_record_matches_a_zero_padded_prefix_regardless_of_type_directory() {
        let store = MapStore::seeded(&[("/bundle/bdr/0007-behavior.md", NEW_RECORD)]);

        let found =
            find_record(&store, Path::new("/bundle"), 7).expect("find_record should succeed");

        assert_eq!(found, PathBuf::from("/bundle/bdr/0007-behavior.md"));
    }
}
