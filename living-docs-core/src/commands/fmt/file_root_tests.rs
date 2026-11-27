use super::*;
use crate::test_support::WritableMapStore as MapStore;

fn scratch_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("living-docs-fmt-file-root-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, "plain text, not a bundle\n").unwrap();
    path
}

fn refusal(result: Result<Outcome, String>) -> Option<String> {
    result.err()
}

#[test]
fn a_regular_file_bundle_root_is_refused_by_the_bundle_root_entry() {
    let root = scratch_file("docs");
    let store = MapStore::seeded(&[]);

    let result = outcome_for_bundle(&store, &root, false, &Seeds::default());

    let expected = format!("bundle root not found: {}", root.display());
    assert_eq!(refusal(result), Some(expected));
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}

#[test]
fn a_missing_bundle_root_is_refused_by_the_bundle_root_entry() {
    let root = std::env::temp_dir().join("living-docs-fmt-file-root-absent");
    let store = MapStore::seeded(&[]);

    let result = outcome_for_bundle(&store, &root, false, &Seeds::default());

    let expected = format!("bundle root not found: {}", root.display());
    assert_eq!(refusal(result), Some(expected));
}

#[test]
fn an_explicit_record_file_is_still_reported_by_outcome() {
    let target = scratch_file("0001-doc.md");
    let text = "---\ntitle: Doc\ntype: ADR\ndescription: A doc.\n---\n\n# Doc\n\nBody.\n";
    let store = MapStore::seeded(&[(target.to_str().unwrap(), text)]);

    let result = outcome(&store, &target, true, &Seeds::default());

    assert!(matches!(result, Ok(Outcome::Pending(paths)) if paths == vec![target.clone()]));
    let _ = std::fs::remove_dir_all(target.parent().unwrap());
}

#[test]
fn a_directory_bundle_root_is_handled_like_outcome() {
    let root = std::env::temp_dir();
    let store = MapStore::seeded(&[]);

    let result = outcome_for_bundle(&store, &root, false, &Seeds::default());

    assert!(matches!(result, Ok(Outcome::Rewritten(paths)) if paths.is_empty()));
}
