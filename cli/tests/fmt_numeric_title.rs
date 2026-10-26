mod common;

use common::{
    index_bundle, living_docs, run_check, run_check_advisories, stdout_of, temp_bundle, write,
};
use std::fs;
use std::path::Path;

fn adr(title: &str, heading: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: {title}\ndescription: d\nowner: o\nstatus: Proposed\ntimestamp: 2026-01-01T00:00:00Z\n---\n\n{heading}\n\n## Context\n\nx\n\n## Decision\n\nx\n\n## Consequences\n\nx\n"
    )
}

fn fmt(bundle: &Path) {
    let output = living_docs()
        .args(["fmt", bundle.to_str().unwrap(), "--plain"])
        .output()
        .expect("failed to run living-docs fmt");
    assert!(output.status.success(), "fmt failed");
}

/// Runs `fmt` twice over a record and returns the heading line after the
/// first run, asserting the second run changed nothing and `check` is clean.
fn heading_after_fmt(label: &str, number: &str, title: &str, heading: &str) -> String {
    heading_after_fmt_as(label, number, &adr(title, heading))
}

fn heading_after_fmt_as(label: &str, number: &str, record: &str) -> String {
    let bundle = temp_bundle("fmt-numeric-title", label);
    let rel = format!("adr/{number}-record.md");
    write(&bundle, &rel, record);
    index_bundle(&bundle);
    fmt(&bundle);
    let once = fs::read_to_string(bundle.join(&rel)).unwrap();
    fmt(&bundle);
    assert_eq!(once, fs::read_to_string(bundle.join(&rel)).unwrap());
    let check = run_check(&bundle);
    assert!(check.status.success(), "{}", stdout_of(&check));
    once.lines()
        .find(|l| l.starts_with('#'))
        .unwrap()
        .trim_end_matches('\r')
        .to_owned()
}

#[test]
fn a_digit_only_title_differing_from_the_number_ends_as_the_title_alone() {
    let heading = heading_after_fmt("differs", "0003", "0001", "# Something");
    assert_eq!(heading, "# 0001");
}

#[test]
fn a_title_equal_to_the_first_record_number_ends_as_the_title_alone() {
    let heading = heading_after_fmt("first", "0001", "0001", "# Something");
    assert_eq!(heading, "# 0001");
}

#[test]
fn a_title_equal_to_its_own_number_ends_as_the_title_alone() {
    let heading = heading_after_fmt("own", "0003", "0003", "# Something");
    assert_eq!(heading, "# 0003");
}

#[test]
fn a_numbered_digit_only_title_heading_stays() {
    let heading = heading_after_fmt("keeps", "0003", "0001", "# 0003. 0001");
    assert_eq!(heading, "# 0003. 0001");
}

#[test]
fn a_bare_digit_heading_equal_to_the_title_stays() {
    let heading = heading_after_fmt("bare", "0003", "0003", "# 0003");
    assert_eq!(heading, "# 0003");
}

#[test]
fn a_digit_led_title_with_words_keeps_its_heading() {
    let heading = heading_after_fmt("words", "0003", "2024 roadmap", "# 0003. 2024 roadmap");
    assert_eq!(heading, "# 0003. 2024 roadmap");
}

#[test]
fn a_crlf_record_keeps_its_line_endings_and_the_title_heading() {
    let record = adr("0001", "# Something").replace('\n', "\r\n");
    let heading = heading_after_fmt_as("crlf", "0003", &record);
    assert_eq!(heading, "# 0001");
}

#[test]
fn a_bare_year_heading_disagreeing_with_the_title_is_flagged_and_aligned() {
    let bundle = temp_bundle("fmt-numeric-title", "year");
    let rel = "adr/0003-roadmap.md";
    write(&bundle, rel, &adr("Roadmap", "# 2024"));
    index_bundle(&bundle);
    let advisories = stdout_of(&run_check_advisories(&bundle));
    assert!(advisories.contains("HEADING"), "{advisories}");
    fmt(&bundle);
    let fixed = fs::read_to_string(bundle.join(rel)).unwrap();
    assert!(fixed.contains("\n# Roadmap\n"), "{fixed}");
}

#[test]
fn set_number_leaves_a_bare_digit_heading_unnumbered() {
    let bundle = temp_bundle("fmt-numeric-title", "setnum");
    write(&bundle, "adr/0003-record.md", &adr("0003", "# 0003"));
    index_bundle(&bundle);
    let output = living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap()])
        .args(["set", "adr/0003", "number", "0007"])
        .output()
        .expect("failed to run living-docs set");
    assert!(output.status.success(), "{}", stdout_of(&output));
    let renamed = fs::read_dir(bundle.join("adr"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("0007"))
        .expect("renumbered record");
    let contents = fs::read_to_string(renamed).unwrap();
    assert!(contents.contains("\n# 0003\n"), "{contents}");
}
