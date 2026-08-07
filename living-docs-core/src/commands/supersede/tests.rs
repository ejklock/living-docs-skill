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
    let contents =
        "---\nsupersedes:                 # NNNN of the ADR this replaces, if any\n---\n\n# Body\n";
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

    let found = find_record(&store, Path::new("/bundle"), "7").expect("find_record should succeed");

    assert_eq!(found, PathBuf::from("/bundle/bdr/0007-behavior.md"));
}

#[test]
fn parse_record_reference_accepts_a_bare_number_with_no_qualifier() {
    let (dir, number) = parse_record_reference("0028").expect("bare number must parse");

    assert_eq!(dir, None);
    assert_eq!(number, 28);
}

#[test]
fn parse_record_reference_resolves_a_type_qualifier_through_the_registry() {
    let (dir, number) = parse_record_reference("issue/0028").expect("qualifier must resolve");

    assert_eq!(dir, Some("issues"));
    assert_eq!(number, 28);
}

#[test]
fn parse_record_reference_rejects_an_unknown_type_qualifier() {
    let err = parse_record_reference("glossary/0028")
        .expect_err("an unregistered token must be rejected");

    assert!(err.contains("glossary"), "got: {err}");
    assert!(err.contains("issue"), "got: {err}");
}

#[test]
fn find_record_resolves_a_type_qualified_reference_to_only_that_type() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0028-collision.md", OLD_RECORD),
        ("/bundle/issues/0028-collision.md", NEW_RECORD),
    ]);

    let found = find_record(&store, Path::new("/bundle"), "issue/0028")
        .expect("qualified reference must resolve");

    assert_eq!(found, PathBuf::from("/bundle/issues/0028-collision.md"));
}

#[test]
fn find_record_fails_loud_on_an_unqualified_cross_type_collision_naming_every_candidate() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0028-collision.md", OLD_RECORD),
        ("/bundle/issues/0028-collision.md", NEW_RECORD),
    ]);

    let err = find_record(&store, Path::new("/bundle"), "0028")
        .expect_err("an unqualified cross-type collision must be rejected");

    assert!(err.contains("/bundle/adr/0028-collision.md"), "got: {err}");
    assert!(
        err.contains("/bundle/issues/0028-collision.md"),
        "got: {err}"
    );
    assert!(err.contains("0028"), "got: {err}");
}

#[test]
fn find_record_fails_when_the_named_type_has_no_matching_record() {
    let store = MapStore::seeded(&[("/bundle/adr/0028-decision.md", OLD_RECORD)]);

    let err = find_record(&store, Path::new("/bundle"), "issue/0028")
        .expect_err("a qualifier naming a type with no match must fail");

    assert!(err.contains("issue/0028"), "got: {err}");
}

#[test]
fn supersede_honors_a_type_qualifier_and_leaves_the_colliding_other_type_record_untouched() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0028-old.md", OLD_RECORD),
        ("/bundle/issues/0028-old.md", NEW_RECORD),
        ("/bundle/adr/0029-new.md", NEW_RECORD),
    ]);

    supersede(&store, Path::new("/bundle"), "adr/0028", "0029")
        .expect("supersede with a qualified old reference should succeed");

    let adr_old = store.read(Path::new("/bundle/adr/0028-old.md")).unwrap();
    let issue_old = store.read(Path::new("/bundle/issues/0028-old.md")).unwrap();

    assert!(adr_old.contains("status: Superseded"), "got: {adr_old}");
    assert_eq!(
        issue_old, NEW_RECORD,
        "colliding Issue record must stay byte-identical"
    );
}

#[test]
fn supersede_fails_loud_on_an_ambiguous_unqualified_reference_and_writes_neither_record() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0028-old.md", OLD_RECORD),
        ("/bundle/issues/0028-old.md", NEW_RECORD),
        ("/bundle/adr/0029-new.md", NEW_RECORD),
    ]);

    let err = supersede(&store, Path::new("/bundle"), "0028", "0029")
        .expect_err("an ambiguous old reference must be rejected");

    assert!(err.contains("0028"), "got: {err}");
    let adr_old = store.read(Path::new("/bundle/adr/0028-old.md")).unwrap();
    let issue_old = store.read(Path::new("/bundle/issues/0028-old.md")).unwrap();
    assert_eq!(adr_old, OLD_RECORD, "ADR record must stay untouched");
    assert_eq!(issue_old, NEW_RECORD, "Issue record must stay untouched");
}
