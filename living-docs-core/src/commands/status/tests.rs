use super::*;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

#[test]
fn validate_status_accepts_every_type_s_own_vocabulary() {
    for spec in doc_type::DOC_TYPES {
        for value in spec.status_vocabulary {
            assert!(
                validate_status(value, spec).is_ok(),
                "expected {value} to be valid for {}",
                spec.token
            );
        }
    }
}

#[test]
fn validate_status_rejects_a_value_from_a_different_type_s_vocabulary() {
    let issue_spec = doc_type::spec_for("issue").unwrap();

    let err = validate_status("Proposed", issue_spec)
        .expect_err("an ADR-only value must be rejected for an Issue");

    assert!(err.contains("open"), "got: {err}");
    assert!(err.contains("in-progress"), "got: {err}");
    assert!(err.contains("closed"), "got: {err}");
    assert!(!err.contains("Proposed, Accepted"), "got: {err}");
}

#[test]
fn validate_status_rejects_superseded_case_insensitively_with_a_supersede_hint_for_every_type() {
    for spec in doc_type::DOC_TYPES {
        for value in ["Superseded", "superseded", "SUPERSEDED"] {
            let err = validate_status(value, spec)
                .expect_err("Superseded must be rejected for every type");
            assert!(err.contains("living-docs supersede"), "got: {err}");
        }
    }
}

#[test]
fn validate_status_rejects_an_unknown_value_and_names_the_records_own_type_s_vocabulary() {
    let adr_spec = doc_type::spec_for("adr").unwrap();

    let err = validate_status("Acepted", adr_spec).expect_err("typo status must be rejected");

    assert!(err.contains("Proposed"), "got: {err}");
    assert!(err.contains("Accepted"), "got: {err}");
    assert!(err.contains("Deprecated"), "got: {err}");
}

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

const RECORD: &str =
    "---\ntype: ADR\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# Record\n";

#[test]
fn status_sets_the_status_field_and_preserves_the_rest_of_the_record() {
    let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

    status(&store, Path::new("/bundle"), "0001", "Accepted").expect("status should succeed");

    let updated = store.read(Path::new("/bundle/adr/0001-record.md")).unwrap();
    assert!(updated.contains("status: Accepted"), "got: {updated}");
    assert!(updated.contains("# Record\n"), "got: {updated}");
    assert!(updated.contains("supersedes:\n"), "got: {updated}");
}

#[test]
fn status_rejects_superseded_without_touching_the_store() {
    let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

    let err = status(&store, Path::new("/bundle"), "0001", "Superseded")
        .expect_err("Superseded must be rejected");

    assert!(err.contains("living-docs supersede"), "got: {err}");
    let unchanged = store.read(Path::new("/bundle/adr/0001-record.md")).unwrap();
    assert_eq!(unchanged, RECORD);
}

#[test]
fn status_fails_when_the_store_lists_no_record_for_a_number() {
    let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

    let err = status(&store, Path::new("/bundle"), "0099", "Accepted")
        .expect_err("status must fail when the record cannot be found");

    assert!(err.contains("no record found for 0099"), "got: {err}");
}

const ISSUE_RECORD_0028: &str = "---\ntype: Issue\nstatus: open\n---\n\n# Record\n";
const ADR_RECORD_0028: &str =
    "---\ntype: ADR\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# Record\n";

#[test]
fn status_resolves_a_type_qualified_reference_to_only_that_type_across_a_number_collision() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0028-collision.md", ADR_RECORD_0028),
        ("/bundle/issues/0028-collision.md", ISSUE_RECORD_0028),
    ]);

    status(&store, Path::new("/bundle"), "issue/0028", "closed")
        .expect("a type-qualified reference must resolve");

    let issue = store
        .read(Path::new("/bundle/issues/0028-collision.md"))
        .unwrap();
    let adr = store
        .read(Path::new("/bundle/adr/0028-collision.md"))
        .unwrap();
    assert!(issue.contains("status: closed"), "got: {issue}");
    assert_eq!(
        adr, ADR_RECORD_0028,
        "the colliding ADR must stay byte-identical"
    );
}

#[test]
fn status_fails_loud_on_an_unqualified_cross_type_collision_and_writes_no_file() {
    let store = MapStore::seeded(&[
        ("/bundle/adr/0028-collision.md", ADR_RECORD_0028),
        ("/bundle/issues/0028-collision.md", ISSUE_RECORD_0028),
    ]);

    let err = status(&store, Path::new("/bundle"), "0028", "closed")
        .expect_err("an unqualified cross-type collision must be rejected");

    assert!(err.contains("/bundle/adr/0028-collision.md"), "got: {err}");
    assert!(
        err.contains("/bundle/issues/0028-collision.md"),
        "got: {err}"
    );
    let adr = store
        .read(Path::new("/bundle/adr/0028-collision.md"))
        .unwrap();
    let issue = store
        .read(Path::new("/bundle/issues/0028-collision.md"))
        .unwrap();
    assert_eq!(
        adr, ADR_RECORD_0028,
        "no file may be written on an ambiguous reference"
    );
    assert_eq!(
        issue, ISSUE_RECORD_0028,
        "no file may be written on an ambiguous reference"
    );
}

const ISSUE_RECORD: &str = "---\ntype: Issue\nstatus: open\n---\n\n# Record\n";

#[test]
fn status_validates_against_the_records_own_type_not_a_fixed_global_list() {
    let store = MapStore::seeded(&[("/bundle/issues/0001-record.md", ISSUE_RECORD)]);

    status(&store, Path::new("/bundle"), "0001", "in-progress")
        .expect("in-progress is a valid Issue status");

    let updated = store
        .read(Path::new("/bundle/issues/0001-record.md"))
        .unwrap();
    assert!(updated.contains("status: in-progress"), "got: {updated}");
}

#[test]
fn status_rejects_an_adr_only_value_for_an_issue_record() {
    let store = MapStore::seeded(&[("/bundle/issues/0001-record.md", ISSUE_RECORD)]);

    let err = status(&store, Path::new("/bundle"), "0001", "Proposed")
        .expect_err("Proposed is not a valid Issue status");

    assert!(err.contains("open"), "got: {err}");
    assert!(err.contains("in-progress"), "got: {err}");
    assert!(err.contains("closed"), "got: {err}");
    let unchanged = store
        .read(Path::new("/bundle/issues/0001-record.md"))
        .unwrap();
    assert_eq!(unchanged, ISSUE_RECORD);
}

#[test]
fn status_fails_when_the_records_type_frontmatter_is_unrecognized() {
    let record = "---\ntype: Glossary\nstatus: Active\n---\n\n# Record\n";
    let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", record)]);

    let err = status(&store, Path::new("/bundle"), "0001", "Accepted")
        .expect_err("an unrecognized type must be rejected");

    assert!(err.contains("Glossary"), "got: {err}");
}

#[test]
fn run_returns_the_success_exit_code_when_status_is_set() {
    let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

    let code = run(&store, Path::new("/bundle"), "0001", "Accepted");

    assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}

#[test]
fn run_returns_a_non_success_exit_code_for_an_unknown_status() {
    let store = MapStore::seeded(&[("/bundle/adr/0001-record.md", RECORD)]);

    let code = run(&store, Path::new("/bundle"), "0001", "Acepted");

    assert_ne!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
}
