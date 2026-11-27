use super::*;
use crate::test_support::WritableMapStore as MapStore;
use std::time::{SystemTime, UNIX_EPOCH};

/// A scratch directory unique to one test, removed when it goes out of scope
/// so a failing assert still cleans up.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        Self(std::env::temp_dir().join(format!("living-docs-fmt-file-root-{name}-{nanos}")))
    }

    fn file(&self, name: &str) -> PathBuf {
        std::fs::create_dir_all(&self.0).unwrap();
        let path = self.0.join(name);
        std::fs::write(&path, "plain text, not a bundle\n").unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn refusal(result: Result<Outcome, String>) -> Option<String> {
    result.err()
}

#[test]
fn a_regular_file_bundle_root_is_refused_by_the_bundle_root_entry() {
    let scratch = Scratch::new("regular");
    let root = scratch.file("docs");
    let store = MapStore::seeded(&[]);

    let result = outcome_for_bundle(&store, &root, false, &Seeds::default());

    let expected = format!("bundle root not found: {}", root.display());
    assert_eq!(refusal(result), Some(expected));
}

#[test]
fn a_missing_bundle_root_is_refused_by_the_bundle_root_entry() {
    let scratch = Scratch::new("absent");
    let root = scratch.0.join("docs");
    let store = MapStore::seeded(&[]);

    let result = outcome_for_bundle(&store, &root, false, &Seeds::default());

    let expected = format!("bundle root not found: {}", root.display());
    assert_eq!(refusal(result), Some(expected));
}

#[test]
fn an_explicit_record_file_is_still_reported_by_outcome() {
    let scratch = Scratch::new("record");
    let target = scratch.file("0001-doc.md");
    let text = "---\ntitle: Doc\ntype: ADR\ndescription: A doc.\n---\n\n# Doc\n\nBody.\n";
    let store = MapStore::seeded(&[(target.to_str().unwrap(), text)]);

    let result = outcome(&store, &target, true, &Seeds::default());

    assert!(matches!(result, Ok(Outcome::Pending(paths)) if paths == vec![target.clone()]));
}

#[test]
fn a_directory_bundle_root_is_handled_like_outcome() {
    let root = std::env::temp_dir();
    let store = MapStore::seeded(&[]);

    let result = outcome_for_bundle(&store, &root, false, &Seeds::default());

    assert!(matches!(result, Ok(Outcome::Rewritten(paths)) if paths.is_empty()));
}
