use super::*;
use crate::record::format_scalar;
use crate::test_support::WritableMapStore as MapStore;

const ADR: &str = "---\ntype: ADR\ntitle: A Decision\ndescription: <One sentence — the decision and its scope.>\nstatus: Proposed\nsupersedes:\nsuperseded_by:\n---\n\n# A Decision\n";

fn store() -> MapStore {
    MapStore::seeded(&[("/bundle/adr/0001-a-decision.md", ADR)])
}

#[test]
fn set_status_updates_the_field_within_the_type_vocabulary() {
    let store = store();
    apply(&store, Path::new("/bundle"), "0001", "status", "Accepted").expect("valid status");
    assert!(store
        .contents("/bundle/adr/0001-a-decision.md")
        .contains("status: Accepted\n"));
}

#[test]
fn set_status_rejects_a_value_outside_the_type_vocabulary_and_writes_nothing() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "status", "Ratified")
        .expect_err("Ratified is not an ADR status");
    assert!(err.contains("not a valid status"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}

#[test]
fn set_status_reserves_superseded_for_the_supersede_verb() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "status", "Superseded")
        .expect_err("Superseded is set only via supersede");
    assert!(err.contains("living-docs supersede"), "got: {err}");
}

#[test]
fn set_status_deprecated_adds_the_deprecated_callout() {
    let store = store();
    apply(&store, Path::new("/bundle"), "0001", "status", "Deprecated").expect("valid status");
    let contents = store.contents("/bundle/adr/0001-a-decision.md");
    assert!(
        contents.contains(
            "> **DEPRECATED — do not act on this record.** It has no successor. \
             Run `living-docs read` for what is in force.\n\n# A Decision"
        ),
        "got: {contents}"
    );
}

#[test]
fn set_status_back_to_active_removes_the_callout_and_leaves_the_body_byte_identical() {
    let store = store();
    apply(&store, Path::new("/bundle"), "0001", "status", "Deprecated").expect("valid status");
    apply(&store, Path::new("/bundle"), "0001", "status", "Accepted").expect("valid status");

    let contents = store.contents("/bundle/adr/0001-a-decision.md");
    let original_body = extract_record(Path::new("/bundle/adr/0001-a-decision.md"), ADR).body;
    let final_body = extract_record(Path::new("/bundle/adr/0001-a-decision.md"), &contents).body;

    assert!(!contents.contains("DEPRECATED"), "got: {contents}");
    assert_eq!(final_body, original_body, "got: {contents}");
}

#[test]
fn set_description_quotes_and_replaces_the_placeholder() {
    let store = store();
    apply(
        &store,
        Path::new("/bundle"),
        "0001",
        "description",
        "Caching: a deep dive",
    )
    .expect("any sentence is accepted");
    assert!(store
        .contents("/bundle/adr/0001-a-decision.md")
        .contains(&format!(
            "description: {}\n",
            format_scalar("Caching: a deep dive")
        )));
}

#[test]
fn set_owner_accepts_any_string() {
    let store = store();
    apply(
        &store,
        Path::new("/bundle"),
        "0001",
        "owner",
        "carol@example.com",
    )
    .expect("any owner");
    assert!(store
        .contents("/bundle/adr/0001-a-decision.md")
        .contains("owner: carol@example.com\n"));
}

#[test]
fn set_rejects_an_unknown_field() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "priority", "urgent")
        .expect_err("priority is not a settable field");
    assert!(err.contains("not a settable field"), "got: {err}");
    assert!(err.contains("section with --from"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}

#[test]
fn set_rejects_a_misspelled_section_key() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0001", "sections", "x")
        .expect_err("sections is not a settable field");
    assert!(err.contains("not a settable field"), "got: {err}");
    assert!(err.contains("section with --from"), "got: {err}");
}

#[test]
fn set_fails_when_the_record_cannot_be_found_and_leaves_it_unchanged() {
    let store = store();
    let err = apply(&store, Path::new("/bundle"), "0099", "status", "Accepted")
        .expect_err("no such record");
    assert!(err.contains("no record found for 0099"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}

#[test]
fn set_section_writes_the_text_under_the_heading_and_nothing_else() {
    let store = store();
    apply_section(&store, Path::new("/bundle"), "0001", "Notes", "n\n").expect("section");
    let written = store.contents("/bundle/adr/0001-a-decision.md");
    assert!(
        written.ends_with("# A Decision\n\n## Notes\n\nn\n"),
        "{written}"
    );
    assert!(written.contains("status: Proposed\n"));
}

#[test]
fn set_section_on_an_unknown_reference_writes_nothing() {
    let store = store();
    let err = apply_section(&store, Path::new("/bundle"), "0099", "Notes", "n\n")
        .expect_err("no such record");
    assert!(err.contains("no record found for 0099"), "got: {err}");
    assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
}

const ISSUE: &str = "---\ntype: Issue\ntitle: A Task\ndescription: d\nstatus: open\nlabels: [old]\n---\n\n# A Task\n";
const ISSUE_PATH: &str = "/bundle/issues/0001-a-task.md";

fn issue_store() -> MapStore {
    MapStore::seeded(&[(ISSUE_PATH, ISSUE)])
}

#[test]
fn set_labels_replaces_an_existing_list_with_trimmed_ordered_items() {
    let store = issue_store();
    apply(&store, Path::new("/bundle"), "0001", "labels", " b, ,a ,").expect("labels");
    let written = store.contents(ISSUE_PATH);
    assert!(written.contains("labels: [b, a]\n"), "{written}");
    assert_eq!(written.matches("labels:").count(), 1);
}

#[test]
fn set_blocked_by_with_an_empty_value_writes_an_empty_list() {
    let store = issue_store();
    apply(&store, Path::new("/bundle"), "0001", "blocked_by", "").expect("empty list");
    assert!(store.contents(ISSUE_PATH).contains("blocked_by: []\n"));
}

#[test]
fn set_tracker_quotes_a_value_a_plain_scalar_would_misread() {
    let store = issue_store();
    apply(&store, Path::new("/bundle"), "0001", "tracker", "#12").expect("tracker");
    assert!(store.contents(ISSUE_PATH).contains("tracker: \"#12\"\n"));
}

#[test]
fn set_tracker_rejects_a_blank_value_and_writes_nothing() {
    let store = issue_store();
    let err =
        apply(&store, Path::new("/bundle"), "0001", "tracker", "  ").expect_err("blank tracker");
    assert!(err.contains("tracker"), "got: {err}");
    assert_eq!(store.contents(ISSUE_PATH), ISSUE);
}

#[test]
fn set_labels_rejects_a_bracketed_item_naming_the_key() {
    let store = issue_store();
    let err = apply(&store, Path::new("/bundle"), "0001", "labels", "a,[b]")
        .expect_err("brackets unsupported");
    assert!(err.contains("labels"), "got: {err}");
    assert_eq!(store.contents(ISSUE_PATH), ISSUE);
}

#[test]
fn set_labels_refuses_a_block_style_list_it_cannot_replace_cleanly() {
    let block = ISSUE.replace("labels: [old]", "labels:\n  - old\n  - older");
    let store = MapStore::seeded(&[(ISSUE_PATH, block.as_str())]);
    let err = apply(&store, Path::new("/bundle"), "0001", "labels", "c")
        .expect_err("block list left behind would break the frontmatter");
    assert!(err.contains("labels"), "got: {err}");
    assert_eq!(store.contents(ISSUE_PATH), block);
}

#[test]
fn set_tracker_quotes_a_value_a_plain_scalar_would_truncate() {
    let store = issue_store();
    apply(&store, Path::new("/bundle"), "0001", "tracker", "12 #x").expect("comment opener");
    assert!(store.contents(ISSUE_PATH).contains("tracker: \"12 #x\"\n"));
}

#[test]
fn set_issue_only_keys_are_refused_on_an_adr_naming_the_issue_type() {
    for key in ["tracker", "labels", "blocked_by"] {
        let store = store();
        let err = apply(&store, Path::new("/bundle"), "0001", key, "x")
            .expect_err("issue-only key on an ADR");
        assert!(err.contains(key) && err.contains("Issue"), "got: {err}");
        assert_eq!(store.contents("/bundle/adr/0001-a-decision.md"), ADR);
    }
}
