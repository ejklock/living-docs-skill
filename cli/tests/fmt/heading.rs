use super::repair::{body_of, fmt_record};

const NO_IDENTITY: &[(&str, &str)] = &[];
const HAND: &str = "adr/0003-hand.md";

fn adr(title_line: &str, status: &str, body: &str) -> String {
    format!("---\ntype: ADR\n{title_line}description: d\nstatus: {status}\n---\n\n{body}")
}

fn live(body: &str) -> String {
    adr("title: Hand\n", "Accepted", body)
}

fn body_after_fmt(record: &str, rel: &str) -> String {
    body_of(&fmt_record(NO_IDENTITY, record, rel)).to_owned()
}

#[test]
fn heading_a_section_heading_without_a_numbered_title_heading_is_left_alone() {
    let body = "## Context\n\nText.\n";
    assert_eq!(body_after_fmt(&live(body), HAND), body);
}

#[test]
fn heading_only_the_numbered_line_changes_below_an_earlier_section_heading() {
    let body = "## Context\n\n# 0003. Other\n\nText.\n";
    let expected = "## Context\n\n# 0003. Hand\n\nText.\n";
    assert_eq!(body_after_fmt(&live(body), HAND), expected);
}

#[test]
fn heading_a_fenced_numbered_line_before_the_real_one_is_not_touched() {
    let body = "```\n# 0003. Fake\n```\n\n# 0003. Other\n";
    let expected = "```\n# 0003. Fake\n```\n\n# 0003. Hand\n";
    assert_eq!(body_after_fmt(&live(body), HAND), expected);
}

#[test]
fn heading_carrying_another_records_number_is_unchanged() {
    let body = "# 0004. Other\n\nText.\n";
    assert_eq!(body_after_fmt(&live(body), HAND), body);
}

#[test]
fn heading_without_a_title_and_without_a_numbered_heading_seeds_nothing() {
    let record = adr("", "Accepted", "## Context\n\nText.\n");
    let after = fmt_record(NO_IDENTITY, &record, HAND);
    assert!(!after.contains("title: Context"), "{after}");
    assert_eq!(body_of(&after), "## Context\n\nText.\n");
}

#[test]
fn heading_unnumbered_level_one_takes_the_title_when_no_numbered_heading_exists() {
    let view = "---\ntype: Architecture View\ntitle: Overview\ndescription: d\n---\n\n## Context\n\n# Something else\n\nText.\n";
    assert_eq!(
        body_after_fmt(view, "architecture/overview.md"),
        "## Context\n\n# Overview\n\nText.\n"
    );
}

#[test]
fn heading_the_numbered_heading_wins_over_an_unnumbered_level_one() {
    let body = "# Unnumbered\n\n# 0003. Other\n";
    assert_eq!(
        body_after_fmt(&live(body), HAND),
        "# Unnumbered\n\n# 0003. Hand\n"
    );
}

#[test]
fn heading_a_title_with_edge_spaces_aligns_trimmed_and_stays_in_the_frontmatter() {
    let record = adr("title: \"  Hand  \"\n", "Accepted", "# 0009. Other\n");
    let after = fmt_record(NO_IDENTITY, &record, "adr/0009-g.md");
    assert_eq!(body_of(&after), "# 0009. Hand\n");
    assert!(after.contains("title: \"  Hand  \"\n"), "{after}");
}

/// Whether `living-docs check` reports a HEADING advisory for `bundle`.
fn check_disagrees(bundle: &std::path::Path) -> bool {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_living-docs"));
    let out = cmd.args(["check", "--advisories", "--plain"]).arg(bundle);
    super::stdout_of(&out.output().unwrap())
        .to_lowercase()
        .contains("disagrees with title")
}

/// Checks `record`, runs fmt, checks again, and returns the verdicts with the
/// formatted contents and the contents after a second fmt.
fn check_fmt_check(label: &str, rel: &str, record: &str) -> (bool, bool, String, String) {
    let bundle = super::temp_bundle(label);
    super::write(&bundle, rel, record);
    let before = check_disagrees(&bundle);
    super::run_fmt(&bundle);
    let formatted = std::fs::read_to_string(bundle.join(rel)).unwrap();
    let after = check_disagrees(&bundle);
    super::run_fmt(&bundle);
    let again = std::fs::read_to_string(bundle.join(rel)).unwrap();
    (before, after, formatted, again)
}

#[test]
fn heading_check_reports_no_drift_after_fmt_on_a_live_numbered_record() {
    let (before, after, _, _) =
        check_fmt_check("heading-check", HAND, &live("# 0003. Other\n\nText.\n"));
    assert!(before, "drift must be reported before fmt");
    assert!(!after, "no drift after fmt");
}

#[test]
fn heading_check_and_fmt_agree_on_an_unnumbered_level_one() {
    let record = "---\ntype: Research\ntitle: Q\ndescription: d\n---\n\n# Different question\n";
    let (before, after, formatted, _) = check_fmt_check("heading-l1", "research/0001-q.md", record);
    assert!(before, "drift must be reported before fmt");
    assert!(!after, "no drift after fmt");
    assert_eq!(body_of(&formatted), "# Q\n");
}

#[test]
fn heading_a_context_only_body_has_no_title_heading_and_no_advisory() {
    let record = "---\ntype: Research\ntitle: Q\ndescription: d\n---\n\n## Context\n";
    let (before, after, formatted, _) = check_fmt_check("heading-l2", "research/0001-q.md", record);
    assert!(!before && !after);
    assert_eq!(body_of(&formatted), "## Context\n");
}

#[test]
fn heading_with_edge_spaces_in_the_title_passes_check_and_a_second_fmt_writes_nothing() {
    let record = adr("title: \"  Hand  \"\n", "Accepted", "# 0009. Other\n");
    let (before, after, formatted, again) =
        check_fmt_check("heading-trim", "adr/0009-g.md", &record);
    assert!(before, "drift must be reported before fmt");
    assert!(!after, "no drift after fmt");
    assert_eq!(formatted, again);
}

#[test]
fn title_empty_in_the_frontmatter_is_seeded_without_doubling_the_number() {
    for empty in ["title: \"\"\n", "title: \"   \"\n"] {
        let record = adr(empty, "Accepted", "# 0001. Real heading\n\nText.\n");
        let after = fmt_record(NO_IDENTITY, &record, "adr/0001-real.md");
        assert!(after.contains("\ntitle: Real heading\n"), "{after}");
        assert_eq!(body_of(&after), "# 0001. Real heading\n\nText.\n");
    }
}

#[test]
fn title_is_seeded_from_an_unnumbered_level_one_heading() {
    let record = "---\ntype: Research\ndescription: d\n---\n\n# Pergunta\n";
    let after = fmt_record(NO_IDENTITY, record, "research/0001-pergunta.md");
    assert!(after.contains("\ntitle: Pergunta\n"), "{after}");
    assert_eq!(body_of(&after), "# Pergunta\n");
}

#[test]
fn title_is_not_seeded_from_an_empty_numbered_heading() {
    let record = adr("", "Accepted", "# 0013.\n\nText.\n");
    let after = fmt_record(NO_IDENTITY, &record, "adr/0013-x.md");
    assert!(!after.contains("title: \"\""), "{after}");
    assert!(!after.contains("title: \n"), "{after}");
    assert_eq!(body_of(&after), "# 0013.\n\nText.\n");
}

#[test]
fn retired_record_keeps_its_heading_and_gets_a_title_without_its_number() {
    let record = adr("", "Deprecated", "# 0002. Old\n\nText.\n");
    let after = fmt_record(NO_IDENTITY, &record, "adr/0002-old.md");
    assert!(after.contains("\ntitle: Old\n"), "{after}");
    assert!(after.contains("\n# 0002. Old\n"), "{after}");
}

#[test]
fn title_of_a_deprecated_record_with_a_drifting_heading_is_unchanged() {
    let record = adr("title: Hand\n", "Deprecated", "# 0003. Other\n\nText.\n");
    let after = fmt_record(NO_IDENTITY, &record, HAND);
    assert!(
        after.contains("\n# 0003. Other\n") && after.contains("\ntitle: Hand\n"),
        "{after}"
    );
}
