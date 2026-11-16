use super::*;

fn record(number: u32, title: &str, status: &str, filename: &str) -> Record {
    Record {
        number,
        title: title.to_string(),
        status: status.to_string(),
        filename: filename.to_string(),
        superseded_by: None,
    }
}

fn retired_record(
    number: u32,
    title: &str,
    status: &str,
    filename: &str,
    superseded_by: Option<&str>,
) -> Record {
    Record {
        superseded_by: superseded_by.map(str::to_string),
        ..record(number, title, status, filename)
    }
}

#[test]
fn render_row_matches_the_locked_row_format_for_an_active_record() {
    let record = record(7, "My Title", "Proposed", "0007-my-title.md");
    assert_eq!(
        render_row(&record, &[]),
        "* [0007 — My Title](0007-my-title.md) - Proposed"
    );
}

#[test]
fn render_row_names_the_successor_when_it_resolves_in_the_listing() {
    let old = retired_record(1, "Old", "Superseded", "0001-old.md", Some("0002"));
    let new = record(2, "New", "Accepted", "0002-new.md");
    let all = vec![old, new];

    assert_eq!(
        render_row(&all[0], &all),
        "* [0001 — Old](0001-old.md) - Superseded by [0002](0002-new.md)"
    );
}

#[test]
fn render_row_renders_a_bare_successor_number_when_none_resolves() {
    let old = retired_record(1, "Old", "Superseded", "0001-old.md", Some("0009"));
    let all = vec![old];

    assert_eq!(
        render_row(&all[0], &all),
        "* [0001 — Old](0001-old.md) - Superseded by 0009"
    );
}

#[test]
fn render_row_renders_the_plain_status_when_a_superseded_record_names_no_successor() {
    let old = retired_record(1, "Old", "Superseded", "0001-old.md", None);
    assert_eq!(
        render_row(&old, &[]),
        "* [0001 — Old](0001-old.md) - Superseded"
    );
}

#[test]
fn render_row_marks_a_deprecated_record_with_no_successor() {
    let record = retired_record(1, "Old", "Deprecated", "0001-old.md", None);
    assert_eq!(
        render_row(&record, &[]),
        "* [0001 — Old](0001-old.md) - Deprecated (no successor)"
    );
}

#[test]
fn render_partitioned_pins_the_adr_active_superseded_byte_shape_with_the_history_note() {
    let records = vec![
        retired_record(1, "Old", "Superseded", "0001-old.md", Some("0002")),
        record(2, "Current", "Accepted", "0002-current.md"),
    ];

    let body = render_partitioned(
        &records,
        "Active",
        "Superseded",
        is_active_status,
        Some(RETIRED_SECTION_NOTE),
    );

    assert_eq!(
        body,
        format!(
            "## Active\n\n* [0002 — Current](0002-current.md) - Accepted\n\n## Superseded\n\n{RETIRED_SECTION_NOTE}\n\n* [0001 — Old](0001-old.md) - Superseded by [0002](0002-current.md)\n"
        )
    );
}

#[test]
fn render_partitioned_emits_only_the_first_heading_when_the_second_bucket_is_empty() {
    let records = vec![record(1, "Only", "open", "0001-only.md")];

    let body = render_partitioned(&records, "Open", "Closed", is_open_status, None);

    assert_eq!(body, "## Open\n\n* [0001 — Only](0001-only.md) - open\n");
}

#[test]
fn render_partitioned_omits_the_note_when_none_is_passed() {
    let records = vec![record(1, "Brewing", "closed", "0001-brewing.md")];

    let body = render_partitioned(&records, "Open", "Closed", is_open_status, None);

    assert!(
        !body.contains("History only"),
        "the Open/Closed axis must never carry the retired-section note: {body}"
    );
}

#[test]
fn is_open_status_treats_closed_done_and_superseded_case_insensitively_as_closed() {
    assert!(!is_open_status("closed"));
    assert!(!is_open_status("Closed"));
    assert!(!is_open_status("done"));
    assert!(!is_open_status("Done"));
    assert!(!is_open_status("Superseded"));
}

#[test]
fn is_open_status_treats_open_in_progress_and_unknown_as_open() {
    assert!(is_open_status("open"));
    assert!(is_open_status("in-progress"));
    assert!(is_open_status("Mystery"));
    assert!(is_open_status(""));
}

#[test]
fn is_active_status_treats_superseded_and_deprecated_as_not_active() {
    assert!(!is_active_status("Superseded"));
    assert!(!is_active_status("Deprecated"));
}

#[test]
fn is_active_status_treats_draft_accepted_and_implemented_as_active() {
    assert!(is_active_status("Draft"));
    assert!(is_active_status("Accepted"));
    assert!(is_active_status("Implemented"));
    assert!(is_active_status("Proposed"));
}

#[test]
fn a_title_holding_any_line_break_renders_on_one_line() {
    let titles = [
        "one\ntwo",
        "one\r\ntwo",
        "one\rtwo",
        "one\n\ntwo",
        "one\r\n\r\ntwo",
        "one\n\r\ntwo",
    ];
    for title in titles {
        assert_eq!(
            render_row(&record(1, title, "open", "0001-one.md"), &[]),
            "* [0001 — one two](0001-one.md) - open",
            "{title:?}"
        );
    }
}

#[test]
fn a_title_without_a_line_break_renders_unchanged() {
    assert_eq!(
        render_row(&record(1, "a  b — c", "open", "0001-a.md"), &[]),
        "* [0001 — a  b — c](0001-a.md) - open"
    );
}

#[test]
fn render_row_writes_a_filename_with_a_space_in_angle_form() {
    let record = record(1, "A", "Accepted", "0001-a b.md");
    assert_eq!(
        render_row(&record, &[]),
        "* [0001 — A](<0001-a b.md>) - Accepted"
    );
}

#[test]
fn render_row_writes_a_spaced_successor_link_in_angle_form() {
    let old = retired_record(1, "Old", "Superseded", "0001-old.md", Some("0002"));
    let all = [
        retired_record(1, "Old", "Superseded", "0001-old.md", Some("0002")),
        record(2, "New", "Accepted", "0002-n w.md"),
    ];
    assert_eq!(
        render_row(&old, &all),
        "* [0001 — Old](0001-old.md) - Superseded by [0002](<0002-n w.md>)"
    );
}
