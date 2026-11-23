use super::*;
use crate::test_support::MapStore;
use std::collections::BTreeMap;

fn store_of(files: &[(&str, &str)]) -> MapStore {
    let mut map = BTreeMap::new();
    for (path, contents) in files {
        map.insert(PathBuf::from(path), (*contents).to_string());
    }
    MapStore { files: map }
}

fn violations(store: &MapStore) -> Vec<(String, String)> {
    let mut reporter = Reporter::new();
    let all: Vec<PathBuf> = store.files.keys().cloned().collect();
    check_placeholders(store, Path::new(""), &all, &mut reporter);
    reporter.into_violations()
}

#[test]
fn a_slot_needs_one_or_more_plain_characters_between_its_braces() {
    assert!(!has_slot("{{}}"));
    assert!(has_slot("{{a}}"));
    assert!(has_slot("text {{A: hint}} more"));
    assert!(!has_slot("{{a\nb}}"));
    assert!(!has_slot("{{a{b}}"));
    assert!(has_slot("{{{a}}"));
    assert!(!has_slot("{{a}"));
}

#[test]
fn an_unfilled_placeholder_in_a_record_body_is_a_violation() {
    let store = store_of(&[(
        "docs/adr/0001-x.md",
        "---\ntype: ADR\n---\n\n## Decision\n\nWe will {{DECISION}}.\n",
    )]);
    assert_eq!(violations(&store).len(), 1);
}

#[test]
fn a_placeholder_shown_inside_code_formatting_is_not_flagged() {
    let store = store_of(&[(
        "docs/adr/0001-x.md",
        "---\ntype: ADR\n---\n\nThe template ships `{{DECISION}}` as a slot.\n",
    )]);
    assert!(violations(&store).is_empty());
}

#[test]
fn a_fully_filled_record_is_clean() {
    let store = store_of(&[(
        "docs/adr/0001-x.md",
        "---\ntype: ADR\n---\n\n## Decision\n\nWe will ship it.\n",
    )]);
    assert!(violations(&store).is_empty());
}

#[test]
fn a_placeholder_carrying_its_hint_is_still_a_violation() {
    let store = store_of(&[(
        "docs/adr/0001-x.md",
        "---\ntype: ADR\n---\n\n## Context\n\n{{CONTEXT: the forces at play; no solution here, <= 80 words}}\n",
    )]);
    assert_eq!(violations(&store).len(), 1);
}
