//! The `decision-table` advisory: a marked decision table in a domain record
//! with a missing or contradictory case. It never fails the gate.

use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, write};

const RECORD: &str = "context/fees.md";

fn domain(rows: &[&str]) -> String {
    format!(
        "---\ntype: Context\ntitle: Fees\ndescription: The fee rules.\ntimestamp: 2026-11-23T10:00:00Z\n---\n\n# Fees\n\n## Rules\n\n### Late fee\n\n<!-- decision-table -->\n| tier (gold, basic) | late (yes, no) | fee |\n|---|---|---|\n{}\n",
        rows.join("\n")
    )
}

fn bundle(label: &str, contents: &str) -> common::Scratch {
    let bundle = common::temp_bundle("decision-table", label);
    write(
        &bundle,
        "index.md",
        "# Index\n\n- [context](context/index.md)\n",
    );
    write(&bundle, RECORD, contents);
    common::index_bundle(&bundle);
    bundle
}

fn check_json(bundle: &Path) -> (Option<i32>, Vec<String>) {
    let output = living_docs()
        .args(["check", "--json", "--"])
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check");
    let json: Value = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    let found = json["advisories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "decision-table")
        .map(|a| a["message"].as_str().unwrap().to_owned())
        .collect();
    (output.status.code(), found)
}

const FULL: [&str; 4] = [
    "| gold | yes | 5 |",
    "| gold | no | 0 |",
    "| basic | yes | 10 |",
    "| basic | no | 0 |",
];

#[test]
fn a_complete_table_raises_nothing() {
    let got = check_json(&bundle("ok", &domain(&FULL)));
    assert_eq!(got, (Some(0), vec![]));
}

#[test]
fn a_table_with_a_missing_case_names_the_combination() {
    let (code, found) = check_json(&bundle("missing", &domain(&FULL[..3])));
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("no row covers tier=basic, late=no"),
        "{found:?}"
    );
}

#[test]
fn a_contradicting_table_names_both_rows() {
    let rows = [FULL[0], FULL[1], FULL[2], FULL[3], "| gold | yes | 7 |"];
    let (code, found) = check_json(&bundle("contradiction", &domain(&rows)));
    assert_eq!(code, Some(0));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("rows 1 and 5"), "{found:?}");
}
