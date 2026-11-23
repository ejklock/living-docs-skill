//! The `anchor-missing` advisory: a local markdown link whose fragment
//! matches no heading of its markdown target. It never moves the exit code.

use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, write};

fn record(title: &str, body: &str) -> String {
    format!(
        "---\ntype: Reference\ntitle: {title}\ndescription: A record.\n---\n\n# {title}\n\n{body}\n"
    )
}

/// A bundle whose `adr/0001-a.md` holds `body`, next to a `context/rules.md`
/// with the given rules body, indexed.
fn bundle_with(label: &str, body: &str, rules: &str) -> common::Scratch {
    let bundle = common::temp_bundle("anchor-missing", label);
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [ADRs](adr/index.md)\n- [Context](context/index.md)\n",
    );
    write(&bundle, "adr/index.md", "# ADRs\n\n- [A](0001-a.md)\n");
    write(
        &bundle,
        "context/index.md",
        "# Context\n\n- [Rules](rules.md)\n",
    );
    write(&bundle, "adr/0001-a.md", &record("A", body));
    write(&bundle, "context/rules.md", &record("Rules", rules));
    write(&bundle, "assets/image.png", "png");
    common::index_bundle(&bundle);
    bundle
}

fn check_json(bundle: &Path) -> (Option<i32>, Value) {
    let output = living_docs()
        .args(["check", "--json", "--"])
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check");
    let json = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    (output.status.code(), json)
}

fn anchor_advisories(json: &Value) -> Vec<&Value> {
    json["advisories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "anchor-missing")
        .collect()
}

fn advised(body: &str, rules: &str) -> (Option<i32>, Vec<String>) {
    let bundle = bundle_with("one", body, rules);
    let (code, json) = check_json(&bundle);
    let messages = anchor_advisories(&json)
        .iter()
        .map(|a| {
            format!(
                "{} {}",
                a["file"].as_str().unwrap(),
                a["message"].as_str().unwrap()
            )
        })
        .collect();
    (code, messages)
}

#[test]
fn a_link_to_an_existing_heading_raises_nothing() {
    let (code, found) = advised("[r](../context/rules.md#late-fee)", "### Late fee\n");
    assert_eq!((code, found), (Some(0), vec![]));
}

#[test]
fn a_link_to_a_missing_heading_is_advised_naming_file_link_and_target() {
    let (code, found) = advised("[r](../context/rules.md#late-fees)", "### Late fee\n");
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("adr/0001-a.md"), "{found:?}");
    assert!(
        found[0].contains("../context/rules.md#late-fees"),
        "{found:?}"
    );
    let (_, target) = found[0].split_once(" of ").expect("names its target");
    assert!(target.ends_with("context/rules.md"), "{found:?}");
}

/// How many `anchor-missing` advisories `living-docs <args>` prints when run
/// from `cwd` over a bundle holding one bad cross-file anchor.
fn advisories_when_run_from(cwd: &Path, args: &[&str]) -> usize {
    let output = living_docs()
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("failed to run living-docs check");
    let json: Value = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    anchor_advisories(&json).len()
}

#[test]
fn the_advisory_fires_however_the_bundle_path_is_spelled() {
    let bundle = bundle_with("spelled", "[r](../context/rules.md#nope)", "### Late fee\n");
    let parent = bundle.parent().unwrap();
    for args in [
        ["check", "--json", "--", "docs"],
        ["check", "--json", "--", "docs/"],
        ["check", "--json", "--", "./docs"],
        ["--docs-dir", "./docs", "check", "--json"],
    ] {
        assert_eq!(advisories_when_run_from(parent, &args), 1, "{args:?}");
    }
    for args in [
        ["check", "--json", "--", "."],
        ["--docs-dir", ".", "check", "--json"],
    ] {
        assert_eq!(advisories_when_run_from(&bundle, &args), 1, "{args:?}");
    }
}

#[test]
fn a_repeated_heading_resolves_its_numbered_suffix() {
    let rules = "## Notes\n\n## Notes\n";
    assert!(advised("[r](../context/rules.md#notes-1)", rules)
        .1
        .is_empty());
    assert_eq!(
        advised("[r](../context/rules.md#notes-2)", rules).1.len(),
        1
    );
}

#[test]
fn a_same_file_fragment_is_judged_against_the_linking_file() {
    let (_, found) = advised("## Present\n\n[a](#missing) [b](#present)", "x\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("#missing"), "{found:?}");
}

#[test]
fn external_non_markdown_and_fragmentless_links_raise_nothing() {
    let body = "[a](https://x.test/a#b) [b](../assets/image.png#x) [c](../context/rules.md)";
    assert!(advised(body, "### Late fee\n").1.is_empty());
}

#[test]
fn a_link_to_a_missing_file_is_a_violation_not_an_anchor_advisory() {
    let bundle = bundle_with("gone", "[r](../context/gone.md#x)", "x\n");
    let (code, json) = check_json(&bundle);
    assert_ne!(code, Some(0));
    assert!(anchor_advisories(&json).is_empty());
    assert!(json["violations"].to_string().contains("broken link"));
}
