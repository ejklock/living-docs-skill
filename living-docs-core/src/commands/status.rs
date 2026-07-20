use crate::commands::supersede::{find_record, parse_record_number, set_frontmatter_fields};
use crate::store::DocStore;
use std::path::Path;
use std::process::ExitCode;

/// The only statuses this verb may set directly. `Superseded` is deliberately
/// excluded — it must go through `supersede`, which also wires the
/// `supersedes`/`superseded_by` links.
const VALID_STATUSES: [&str; 3] = ["Proposed", "Accepted", "Deprecated"];

pub fn run(store: &dyn DocStore, docs_dir: &Path, number: &str, new_status: &str) -> ExitCode {
    match status(store, docs_dir, number, new_status) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("living-docs status: {message}");
            ExitCode::from(2)
        }
    }
}

/// Sets a record's `status:` frontmatter field, reusing `supersede`'s
/// record-resolution ([`find_record`]) and frontmatter-mutation
/// ([`set_frontmatter_fields`]) helpers rather than duplicating them (lesson
/// 3717). `new_status` is validated before any read/write, so an invalid
/// value never reaches the frontmatter writer or partially mutates a file.
fn status(
    store: &dyn DocStore,
    docs_dir: &Path,
    number: &str,
    new_status: &str,
) -> Result<(), String> {
    validate_status(new_status)?;
    let record_number = parse_record_number(number)?;
    let path = find_record(store, docs_dir, record_number)?;
    set_frontmatter_fields(store, &path, &[("status", new_status.to_string())])
}

fn validate_status(new_status: &str) -> Result<(), String> {
    if VALID_STATUSES.contains(&new_status) {
        return Ok(());
    }
    if new_status.eq_ignore_ascii_case("superseded") {
        return Err(
            "'Superseded' must be set via `living-docs supersede <old> <new>`, which also wires the supersedes/superseded_by links".to_string(),
        );
    }
    Err(format!(
        "'{new_status}' is not a valid status; expected one of {}",
        VALID_STATUSES.join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn validate_status_accepts_every_known_value() {
        for value in VALID_STATUSES {
            assert!(
                validate_status(value).is_ok(),
                "expected {value} to be valid"
            );
        }
    }

    #[test]
    fn validate_status_rejects_superseded_case_insensitively_with_a_supersede_hint() {
        for value in ["Superseded", "superseded", "SUPERSEDED"] {
            let err = validate_status(value).expect_err("Superseded must be rejected");
            assert!(err.contains("living-docs supersede"), "got: {err}");
        }
    }

    #[test]
    fn validate_status_rejects_an_unknown_value_and_lists_valid_ones() {
        let err = validate_status("Acepted").expect_err("typo status must be rejected");
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
}
