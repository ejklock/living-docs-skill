use super::*;
use crate::test_support::WritableMapStore as MapStore;

#[test]
fn expected_superseded_links_the_resolved_successor() {
    let line = expected(Some("Superseded"), Some("0002-new-record.md")).unwrap();
    assert_eq!(
        line,
        "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002-new-record.md). \
         Run `living-docs read records` for what is in force."
    );
}

#[test]
fn expected_superseded_falls_back_to_the_bare_number() {
    let line = expected(Some("superseded"), Some("0002")).unwrap();
    assert_eq!(
        line,
        "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002). \
         Run `living-docs read records` for what is in force."
    );
}

#[test]
fn expected_deprecated_has_no_successor_link() {
    let line = expected(Some("Deprecated"), None).unwrap();
    assert_eq!(
        line,
        "> **DEPRECATED — do not act on this record.** It has no successor. \
         Run `living-docs read records` for what is in force."
    );
}

#[test]
fn expected_is_none_for_active_or_absent_status() {
    assert_eq!(expected(Some("Proposed"), None), None);
    assert_eq!(expected(Some("Accepted"), None), None);
    assert_eq!(expected(None, None), None);
}

#[test]
fn leading_detects_either_marker() {
    assert!(leading("> **SUPERSEDED — do not act on this record.**\n\n# Title").is_some());
    assert!(leading("> **DEPRECATED — do not act on this record.**\n\n# Title").is_some());
}

#[test]
fn leading_ignores_a_non_callout_first_line() {
    assert_eq!(leading("# Title\n\nBody"), None);
}

#[test]
fn leading_skips_blank_lines_before_the_first_content_line() {
    assert!(leading("\n\n> **SUPERSEDED — banner**\n\nBody").is_some());
}

const ACTIVE_BODY: &str = "# Title\n\nBody text.\n";
const SUPERSEDED_LINE: &str =
    "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002.md). \
     Run `living-docs read records` for what is in force.";

#[test]
fn reconcile_body_adds_a_missing_callout() {
    let reconciled = reconcile_body(ACTIVE_BODY, Some(SUPERSEDED_LINE));
    assert_eq!(reconciled, format!("{SUPERSEDED_LINE}\n\n{ACTIVE_BODY}"));
}

#[test]
fn reconcile_body_replaces_a_stale_callout() {
    let stale = format!("{SUPERSEDED_LINE}\n\n{ACTIVE_BODY}");
    let fresh_line = "> **DEPRECATED — do not act on this record.** It has no successor. \
                       Run `living-docs read records` for what is in force.";
    let reconciled = reconcile_body(&stale, Some(fresh_line));
    assert_eq!(reconciled, format!("{fresh_line}\n\n{ACTIVE_BODY}"));
}

#[test]
fn reconcile_body_removes_a_callout_when_none_is_expected() {
    let with_callout = format!("{SUPERSEDED_LINE}\n\n{ACTIVE_BODY}");
    let reconciled = reconcile_body(&with_callout, None);
    assert_eq!(reconciled, ACTIVE_BODY);
}

#[test]
fn reconcile_body_is_idempotent() {
    let once = reconcile_body(ACTIVE_BODY, Some(SUPERSEDED_LINE));
    let twice = reconcile_body(&once, Some(SUPERSEDED_LINE));
    assert_eq!(once, twice);
}

#[test]
fn reconcile_body_leaves_an_already_correct_body_untouched() {
    let already = format!("{SUPERSEDED_LINE}\n\n{ACTIVE_BODY}");
    assert_eq!(reconcile_body(&already, Some(SUPERSEDED_LINE)), already);
}

fn store_with(files: &[(&str, &str)]) -> MapStore {
    MapStore::seeded(files)
}

#[test]
fn successor_filename_resolves_a_sibling_with_a_slug() {
    let store = store_with(&[
        ("docs/adr/0001-old.md", "---\ntype: adr\n---\n\n# Old\n"),
        (
            "docs/adr/0002-new-record.md",
            "---\ntype: adr\n---\n\n# New\n",
        ),
    ]);
    let resolved = successor_filename(&store, Path::new("docs/adr/0001-old.md"), "0002");
    assert_eq!(resolved, "0002-new-record.md");
}

#[test]
fn successor_filename_resolves_a_bare_sibling() {
    let store = store_with(&[
        ("docs/adr/0001-old.md", "---\ntype: adr\n---\n\n# Old\n"),
        ("docs/adr/0002.md", "---\ntype: adr\n---\n\n# New\n"),
    ]);
    let resolved = successor_filename(&store, Path::new("docs/adr/0001-old.md"), "0002");
    assert_eq!(resolved, "0002.md");
}

#[test]
fn successor_filename_falls_back_to_the_bare_number_when_no_sibling_matches() {
    let store = store_with(&[("docs/adr/0001-old.md", "---\ntype: adr\n---\n\n# Old\n")]);
    let resolved = successor_filename(&store, Path::new("docs/adr/0001-old.md"), "0002");
    assert_eq!(resolved, "0002");
}

fn superseded_record(superseded_by: &str) -> String {
    format!(
        "---\ntype: adr\ntitle: Old\ndescription: d\nstatus: Superseded\nsuperseded_by: {superseded_by}\n---\n\n# Old\n\nBody text.\n"
    )
}

#[test]
fn reconcile_writes_a_missing_callout_onto_a_superseded_record() {
    let store = store_with(&[
        ("docs/adr/0001-old.md", &superseded_record("0002")),
        (
            "docs/adr/0002-new-record.md",
            "---\ntype: adr\n---\n\n# New\n",
        ),
    ]);
    let wrote = reconcile(&store, Path::new("docs/adr/0001-old.md")).unwrap();
    assert!(wrote);

    let contents = store.read(Path::new("docs/adr/0001-old.md")).unwrap();
    assert!(contents.contains(
        "> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002-new-record.md)."
    ));
    assert!(contents.contains("# Old"));
}

#[test]
fn reconcile_is_a_no_op_on_an_already_reconciled_record() {
    let store = store_with(&[
        ("docs/adr/0001-old.md", &superseded_record("0002")),
        (
            "docs/adr/0002-new-record.md",
            "---\ntype: adr\n---\n\n# New\n",
        ),
    ]);
    reconcile(&store, Path::new("docs/adr/0001-old.md")).unwrap();
    let after_first = store.read(Path::new("docs/adr/0001-old.md")).unwrap();

    let wrote_again = reconcile(&store, Path::new("docs/adr/0001-old.md")).unwrap();
    assert!(!wrote_again);
    assert_eq!(
        store.read(Path::new("docs/adr/0001-old.md")).unwrap(),
        after_first
    );
}

#[test]
fn reconcile_removes_a_callout_from_an_active_record() {
    let active = format!(
        "---\ntype: adr\ntitle: Active\ndescription: d\nstatus: Accepted\n---\n\n{SUPERSEDED_LINE}\n\n# Active\n\nBody text.\n"
    );
    let store = store_with(&[("docs/adr/0003-active.md", &active)]);
    let wrote = reconcile(&store, Path::new("docs/adr/0003-active.md")).unwrap();
    assert!(wrote);

    let contents = store.read(Path::new("docs/adr/0003-active.md")).unwrap();
    assert!(!contents.contains("SUPERSEDED"));
    assert!(contents.contains("# Active"));
}
