//! `read --links-to` and `read --impact` through the binary.

mod common;

use common::{living_docs, stdout_of, temp_bundle, write};
use std::path::Path;
use std::process::Output;

fn adr(title: &str, body: &str) -> String {
    format!("---\ntype: ADR\ntitle: {title}\ndescription: d\nstatus: Accepted\n---\n\n{body}\n")
}

/// 0002 and 0003 link 0001; 0004 links 0002.
fn bundle(label: &str) -> std::path::PathBuf {
    let dir = temp_bundle("read-dependents", label);
    write(&dir, "adr/0001-a.md", &adr("Alpha", "root"));
    write(&dir, "adr/0002-b.md", &adr("Beta", "[a](/adr/0001-a.md)"));
    write(&dir, "adr/0003-c.md", &adr("Gamma", "[a](0001-a.md#top)"));
    write(&dir, "adr/0004-d.md", &adr("Delta", "[b](/adr/0002-b.md)"));
    dir
}

fn read(dir: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", dir.to_str().unwrap(), "read"])
        .args(args)
        .output()
        .expect("failed to run living-docs read")
}

fn records(output: &Output) -> Vec<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(&stdout_of(output)).unwrap();
    value["records"].as_array().unwrap().clone()
}

fn summary(output: &Output) -> Vec<(u64, Option<u64>)> {
    records(output)
        .iter()
        .map(|r| (r["number"].as_u64().unwrap(), r["distance"].as_u64()))
        .collect()
}

#[test]
fn links_to_prints_the_direct_linkers_without_a_distance() {
    let dir = bundle("links-to");
    let output = read(&dir, &["--links-to", "adr/0001", "--json"]);
    assert!(output.status.success());
    assert_eq!(summary(&output), vec![(2, None), (3, None)]);
}

#[test]
fn impact_prints_every_dependent_with_its_distance() {
    let dir = bundle("impact");
    let output = read(&dir, &["--impact", "adr/0001", "--json"]);
    assert!(output.status.success());
    assert_eq!(
        summary(&output),
        vec![(2, Some(1)), (3, Some(1)), (4, Some(2))]
    );
}

#[test]
fn impact_in_plain_text_shows_each_distance() {
    let dir = bundle("impact-plain");
    let output = read(&dir, &["--impact", "adr/0001", "--plain"]);
    assert!(output.status.success());
    let text = stdout_of(&output);
    assert!(
        text.contains("Delta") && text.contains("[distance 2]"),
        "{text}"
    );
    assert_eq!(text.matches("[distance 1]").count(), 2, "{text}");
}

#[test]
fn impact_in_full_text_shows_each_distance_under_its_heading() {
    let dir = bundle("impact-full");
    let output = read(&dir, &["--impact", "adr/0001", "--full", "--plain"]);
    assert!(output.status.success());
    let text = stdout_of(&output);
    let beta = text.find("Beta").expect("Beta heading");
    let delta = text.find("Delta").expect("Delta heading");
    assert!(text[beta..delta].contains("_distance 1_"), "{text}");
    assert!(text[delta..].contains("_distance 2_"), "{text}");
    assert_eq!(text.matches("_distance 1_").count(), 2, "{text}");
    assert_eq!(text.matches("_distance 2_").count(), 1, "{text}");
}

#[test]
fn a_record_nothing_links_exits_zero_with_no_records() {
    let dir = bundle("empty");
    let output = read(&dir, &["--impact", "adr/0004", "--json"]);
    assert!(output.status.success());
    assert!(records(&output).is_empty());
}

#[test]
fn an_unknown_reference_exits_two_and_names_it() {
    let dir = bundle("unknown");
    let output = read(&dir, &["--links-to", "adr/9999", "--json"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("9999"), "{stderr}");
}

#[test]
fn conflicting_flags_exit_two() {
    let dir = bundle("conflicts");
    for args in [
        &["--links-to", "adr/0001", "--impact", "adr/0001"][..],
        &["--links-to", "adr/0001", "--topic", "alpha"][..],
        &["--impact", "adr/0001", "--topic", "alpha"][..],
    ] {
        assert_eq!(read(&dir, args).status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn help_lists_both_flags_with_an_example_each() {
    let output = living_docs()
        .args(["read", "--help", "--plain"])
        .output()
        .unwrap();
    let help = stdout_of(&output);
    assert!(help.contains("--links-to <TYPE/NNNN>"), "{help}");
    assert!(help.contains("--impact <TYPE/NNNN>"), "{help}");
    assert!(help.contains("living-docs read --links-to adr/"), "{help}");
    assert!(help.contains("living-docs read --impact adr/"), "{help}");
}
