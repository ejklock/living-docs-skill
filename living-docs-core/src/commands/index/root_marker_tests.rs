use super::root::{ensure_root_index, NoTypeIndex};
use crate::check::{check_violations, collect_md_files};
use crate::store::DocStore;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const BLOCK: &str = "---\nokf_version: \"0.1\"\n---\n\n";
const RULE_AND_PROSE: &str = "---\n# Docs\n\nSome text\n\n---\n\n* [ADRs](adr/)\n";

/// A store over real files that lists the markdown under the bundle.
struct DiskStore;

impl DocStore for DiskStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(collect_md_files(root))
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }
}

fn bundle_with_root(label: &str, root: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let docs = std::env::temp_dir().join(format!("living-docs-marker-{label}-{nanos}"));
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("index.md"), root).unwrap();
    docs
}

/// Runs the root step twice and returns the root after the first run; the
/// second run must write nothing.
fn migrate(label: &str, root: &str) -> (PathBuf, String) {
    let docs = bundle_with_root(label, root);
    let first = ensure_root_index(&DiskStore, &docs, &[], NoTypeIndex::WriteNothing).unwrap();
    assert_eq!(first, Some(docs.join("index.md")));
    let after = fs::read_to_string(docs.join("index.md")).unwrap();
    let second = ensure_root_index(&DiskStore, &docs, &[], NoTypeIndex::WriteNothing).unwrap();
    assert_eq!(second, None);
    assert_eq!(fs::read_to_string(docs.join("index.md")).unwrap(), after);
    (docs, after)
}

#[test]
fn rule_and_prose_root_gets_a_block_above_its_unchanged_bytes() {
    let (docs, after) = migrate("prose", RULE_AND_PROSE);

    assert_eq!(after, format!("{BLOCK}{RULE_AND_PROSE}"));
    let about_root: Vec<_> = check_violations(&DiskStore, &docs)
        .into_iter()
        .filter(|(file, _)| file.ends_with("index.md") && !file.contains("adr"))
        .filter(|(_, message)| message.contains("okf_version") || message.contains("frontmatter"))
        .collect();
    assert!(about_root.is_empty(), "{about_root:?}");
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn sequence_block_gets_a_new_block_above() {
    let (docs, after) = migrate("sequence", "---\n- a\n---\n");

    assert_eq!(after, format!("{BLOCK}---\n- a\n---\n"));
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn comment_only_block_takes_the_key_inside_it() {
    let (docs, after) = migrate("comment", "---\n# note\n---\n");

    assert_eq!(after, "---\nokf_version: \"0.1\"\n# note\n---\n");
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn mapping_without_the_key_gets_it_as_first_line() {
    let (docs, after) = migrate("mapping", "---\ntitle: Docs\n---\n\n# Docs\n");

    assert_eq!(
        after,
        "---\nokf_version: \"0.1\"\ntitle: Docs\n---\n\n# Docs\n"
    );
    let _ = fs::remove_dir_all(&docs);
}

#[test]
fn crlf_rule_and_prose_root_gets_a_crlf_block_above() {
    let crlf = RULE_AND_PROSE.replace('\n', "\r\n");
    let (docs, after) = migrate("crlf", &crlf);

    assert_eq!(after, format!("{}{crlf}", BLOCK.replace('\n', "\r\n")));
    let _ = fs::remove_dir_all(&docs);
}
