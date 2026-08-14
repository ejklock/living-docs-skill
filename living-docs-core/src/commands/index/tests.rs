use super::*;

/// ADR 0026 fitness function B (index half), extended by ADR 0036: the
/// token set `index` regenerates with no explicit type equals the
/// registry's Identity::Numbered plus Identity::Named token set, in the
/// registry's own order.
#[test]
fn all_type_tokens_matches_every_numbered_and_named_registry_token_in_order() {
    assert_eq!(
        all_type_tokens(),
        vec!["adr", "bdr", "prd", "issue", "research", "view"]
    );
}

/// The `constitution` row is a deliberate exclusion, not an oversight:
/// a singleton has no directory index for the bare sweep to regenerate.
#[test]
fn all_type_tokens_excludes_the_constitution_singleton() {
    assert!(!all_type_tokens().contains(&"constitution".to_string()));
}

#[test]
fn numbered_prefix_accepts_four_digit_dash_form() {
    assert_eq!(numbered_prefix("0007-old.md"), Some(7));
}

#[test]
fn numbered_prefix_rejects_index_and_malformed_names() {
    assert_eq!(numbered_prefix("index.md"), None);
    assert_eq!(numbered_prefix("12-old.md"), None);
    assert_eq!(numbered_prefix("abcd-old.md"), None);
}

#[test]
fn render_row_matches_the_locked_row_format() {
    let record = Record {
        number: 7,
        title: "My Title".to_string(),
        status: "Proposed".to_string(),
        filename: "0007-my-title.md".to_string(),
        visibility: "private".to_string(),
    };
    assert_eq!(
        render_row(&record),
        "* [0007 — My Title](0007-my-title.md) - Proposed"
    );
}

#[test]
fn fallback_preamble_is_minimal_for_a_fresh_file() {
    assert_eq!(fallback_preamble("", "adr"), "# ADRs\n\n");
}

#[test]
fn fallback_preamble_wraps_unmarked_existing_content() {
    assert_eq!(
        fallback_preamble("Custom intro.\n", "prd"),
        "Custom intro.\n\n"
    );
}

#[test]
fn find_boundary_offset_locates_the_adr_active_heading() {
    let existing = "# ADRs\n\nIntro.\n\n## Active\n\n* [0001 — X](0001-x.md) - Proposed\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(
        &existing[offset..],
        "## Active\n\n* [0001 — X](0001-x.md) - Proposed\n"
    );
}

#[test]
fn find_boundary_offset_locates_the_first_row_for_non_adr_types() {
    let existing = "# PRDs\n\nIntro.\n\n* [0001 — X](0001-x.md) - Draft\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(&existing[offset..], "* [0001 — X](0001-x.md) - Draft\n");
}

#[test]
fn find_boundary_offset_locates_a_legacy_heading_regardless_of_its_text() {
    let existing = "# Issues\n\nIntro.\n\n## Done\n\n* [0001 — X](0001-x.md) - closed\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(
        &existing[offset..],
        "## Done\n\n* [0001 — X](0001-x.md) - closed\n"
    );
}

#[test]
fn find_boundary_offset_locates_a_hand_maintained_table_header_row() {
    let existing = "# ADRs\n\nIntro.\n\n| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n";
    let offset = find_boundary_offset(existing).unwrap();
    assert_eq!(
        &existing[offset..],
        "| # | Decision | Status |\n|---|---|---|\n| [0001](0001-x.md) | X | Accepted |\n"
    );
}

#[test]
fn is_boundary_line_detects_a_numbered_listing_table_header() {
    assert!(is_boundary_line("| # | Decision | Status |"));
}

#[test]
fn is_boundary_line_detects_a_table_row_whose_first_cell_is_a_record_link() {
    assert!(is_boundary_line("| [0001](0001-x.md) | X | Accepted |"));
    assert!(is_boundary_line("| [0007-legacy-row | X | Accepted |"));
}

#[test]
fn is_boundary_line_ignores_an_unrelated_table_row() {
    assert!(!is_boundary_line("| Some | Other | Row |"));
    assert!(!is_boundary_line("Just prose, not a table at all."));
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
fn render_partitioned_pins_the_adr_active_superseded_byte_shape() {
    let records = vec![
        Record {
            number: 1,
            title: "Old".to_string(),
            status: "Superseded".to_string(),
            filename: "0001-old.md".to_string(),
            visibility: "private".to_string(),
        },
        Record {
            number: 2,
            title: "Current".to_string(),
            status: "Accepted".to_string(),
            filename: "0002-current.md".to_string(),
            visibility: "private".to_string(),
        },
    ];

    let body = render_partitioned(&records, "Active", "Superseded", is_active_status);

    assert_eq!(
        body,
        "## Active\n\n* [0002 — Current](0002-current.md) - Accepted\n\n## Superseded\n\n* [0001 — Old](0001-old.md) - Superseded\n"
    );
}

#[test]
fn render_partitioned_emits_only_the_first_heading_when_the_second_bucket_is_empty() {
    let records = vec![Record {
        number: 1,
        title: "Only".to_string(),
        status: "open".to_string(),
        filename: "0001-only.md".to_string(),
        visibility: "private".to_string(),
    }];

    let body = render_partitioned(&records, "Open", "Closed", is_open_status);

    assert_eq!(body, "## Open\n\n* [0001 — Only](0001-only.md) - open\n");
}
