use super::*;

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

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut files = self.files.borrow_mut();
        let contents = files
            .remove(from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        files.insert(to.to_path_buf(), contents);
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
fn supersede_writes_the_superseded_callout_on_the_old_record_naming_the_new_records_filename_and_leaves_the_new_record_without_one(
) {
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

    assert!(
        old.contains(
            "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002-new.md). \
             Run `living-docs read records` for what is in force.\n\n# Old"
        ),
        "got: {old}"
    );
    assert!(
        !new.contains("SUPERSEDED") && !new.contains("DEPRECATED"),
        "got: {new}"
    );
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

/// AC2/AC3: inserting a previously-absent key lands it in canonical
/// position (right after `description:`, not at the frontmatter block's
/// close) while a multi-line body stays byte-identical.
#[test]
fn set_frontmatter_fields_inserts_an_absent_key_in_canonical_order() {
    let contents = "---\ntype: BDR\ntitle: A Behavior\ndescription: A summary.\n---\n\n## Context\n\nSome body text.\n";
    let store = MapStore::seeded(&[("/bundle/bdr/0001-a-behavior.md", contents)]);
    let path = Path::new("/bundle/bdr/0001-a-behavior.md");

    set_frontmatter_fields(&store, path, &[("supersedes", "0001".to_string())])
        .expect("set_frontmatter_fields should succeed");

    let updated = store.read(path).unwrap();
    assert_eq!(
        updated,
        "---\ntype: BDR\ntitle: A Behavior\ndescription: A summary.\nsupersedes: 0001\n---\n\n## Context\n\nSome body text.\n",
        "got: {updated}"
    );
}

/// AC2: changing an already-canonical, already-present key round-trips
/// byte-identically.
#[test]
fn set_frontmatter_fields_changing_an_existing_key_stays_canonical() {
    let contents =
        "---\ntype: ADR\ntitle: A Decision\ndescription: A summary.\nstatus: Proposed\n---\n\n# Body\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", contents)]);
    let path = Path::new("/bundle/adr/0001-a-decision.md");

    set_frontmatter_fields(&store, path, &[("status", "Accepted".to_string())])
        .expect("set_frontmatter_fields should succeed");

    let updated = store.read(path).unwrap();
    assert_eq!(
        updated,
        "---\ntype: ADR\ntitle: A Decision\ndescription: A summary.\nstatus: Accepted\n---\n\n# Body\n",
        "got: {updated}"
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
