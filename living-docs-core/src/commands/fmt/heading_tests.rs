use crate::commands::set::retitle::{replace_title_heading, title_heading_text};

const FRONT: &str = "---\ntype: ADR\ntitle: Hand\n---\n\n";

#[test]
fn replace_title_heading_keeps_a_crlf_line_ending() {
    let record = format!("{FRONT}# 0003. Other\r\nBody.\r\n");

    let updated = replace_title_heading(&record, Some("0003"), "Hand").unwrap();

    assert_eq!(updated, format!("{FRONT}# 0003. Hand\r\nBody.\r\n"));
}

#[test]
fn replace_title_heading_keeps_a_missing_final_newline() {
    let record = format!("{FRONT}# 0003. Other");

    let updated = replace_title_heading(&record, Some("0003"), "Hand").unwrap();

    assert_eq!(updated, format!("{FRONT}# 0003. Hand"));
}

#[test]
fn replace_title_heading_skips_a_section_heading_and_other_numbers() {
    let record = format!("{FRONT}## Context\n\n# 0004. Other\n");

    assert_eq!(replace_title_heading(&record, Some("0003"), "Hand"), None);
}

#[test]
fn title_heading_text_reads_the_heading_outside_a_fence() {
    let record = format!("{FRONT}```\n# 0003. Fake\n```\n\n## Context\n\n## 0003. Real\n");

    assert_eq!(
        title_heading_text(&record, Some("0003")).as_deref(),
        Some("Real")
    );
}

#[test]
fn the_numbered_heading_wins_over_an_unnumbered_level_one() {
    let record = format!("{FRONT}# Unnumbered\n\n# 0003. Real\n");

    assert_eq!(
        title_heading_text(&record, Some("0003")).as_deref(),
        Some("Real")
    );
}

#[test]
fn an_unnumbered_level_one_is_the_title_when_no_numbered_heading_exists() {
    let record = format!("{FRONT}## Context\n\n# Question\n");

    assert_eq!(
        title_heading_text(&record, None).as_deref(),
        Some("Question")
    );
    assert_eq!(
        replace_title_heading(&record, None, "Q").as_deref(),
        Some(format!("{FRONT}## Context\n\n# Q\n").as_str())
    );
}

#[test]
fn a_level_two_heading_alone_is_no_title_heading() {
    let record = format!("{FRONT}## Context\n");

    assert_eq!(title_heading_text(&record, None), None);
}
