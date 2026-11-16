//! The check rule file names every advisory kind `check` can report, so the
//! guidance a reader gets never lags the registry.

use living_docs_core::check::kinds;
use std::fs;
use std::path::Path;

#[test]
fn the_check_rule_lists_every_advisory_kind() {
    let rule = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills/living-docs/rules/check.md");
    let text = fs::read_to_string(rule).unwrap();
    let unlisted: Vec<&str> = kinds::ALL
        .iter()
        .map(|kind| kind.name)
        .filter(|name| {
            let label = format!("- `{}`", name.to_uppercase());
            !text.lines().any(|line| line.starts_with(&label))
        })
        .collect();
    assert!(unlisted.is_empty(), "no list line for: {unlisted:?}");
}
