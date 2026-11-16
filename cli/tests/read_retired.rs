//! End-to-end `read records` retirement: every record its doc type retires is
//! withheld and counted, and `--all` brings them back.

use serde_json::Value;
use std::path::Path;
use std::process::Output;

mod common;
use common::retired_bundle::{write_bundle, write_linkers};
use common::{living_docs, stdout_of, temp_bundle};

fn read(bundle: &Path, extra: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", bundle.to_str().unwrap(), "read", "records"])
        .args(extra)
        .output()
        .unwrap()
}

fn read_json(bundle: &Path, extra: &[&str]) -> Value {
    let output = read(bundle, &[extra, &["--json"]].concat());
    assert!(output.status.success(), "{output:?}");
    serde_json::from_str(&stdout_of(&output)).unwrap()
}

fn titles(value: &Value) -> Vec<String> {
    value["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["title"].as_str().unwrap().to_string())
        .collect()
}

fn lineage_of_head(value: &Value) -> Value {
    let records = value["records"].as_array().unwrap();
    records.iter().find(|r| r["title"] == "New rule").unwrap()["lineage"].clone()
}

#[test]
fn retired_records_are_withheld_and_counted() {
    let bundle = temp_bundle("read-retired", "withheld");
    write_bundle(&bundle);

    let value = read_json(&bundle, &[]);

    let mut listed = titles(&value);
    listed.sort();
    assert_eq!(
        listed,
        [
            "Accepted rule",
            "New rule",
            "Open one",
            "Statusless",
            "Unregistered"
        ]
    );
    assert_eq!(value["withheld"], 5);
    assert_eq!(lineage_of_head(&value), "supersedes 0003");
}

#[test]
fn all_lists_every_record_and_withholds_none() {
    let bundle = temp_bundle("read-retired", "all");
    write_bundle(&bundle);

    let value = read_json(&bundle, &["--all"]);

    assert_eq!(titles(&value).len(), 10);
    assert_eq!(value["withheld"], 0);
    assert_eq!(lineage_of_head(&value), "supersedes 0003");
}

#[test]
fn topic_finds_a_retired_record_only_with_all() {
    let bundle = temp_bundle("read-retired", "topic");
    write_bundle(&bundle);

    assert!(titles(&read_json(&bundle, &["--topic", "zebrafish"])).is_empty());
    assert_eq!(
        titles(&read_json(&bundle, &["--topic", "zebrafish", "--all"])),
        ["Zebrafish shut"]
    );
}

#[test]
fn contract_and_full_withhold_the_closed_issue_unless_all() {
    let bundle = temp_bundle("read-retired", "modes");
    write_bundle(&bundle);

    for mode in ["--contract", "--full"] {
        let without = read_json(&bundle, &[mode]);
        assert!(
            !titles(&without).contains(&"Zebrafish shut".to_string()),
            "{mode}"
        );
        assert_eq!(without["withheld"], 5, "{mode}");
        let with = read_json(&bundle, &[mode, "--all"]);
        assert!(
            titles(&with).contains(&"Zebrafish shut".to_string()),
            "{mode}"
        );
        assert_eq!(with["withheld"], 0, "{mode}");
    }
}

#[test]
fn dependents_queries_withhold_retired_linkers_unless_all() {
    let bundle = temp_bundle("read-retired", "dependents");
    write_bundle(&bundle);
    write_linkers(&bundle);

    for query in ["--links-to", "--impact"] {
        let without = read_json(&bundle, &[query, "adr/0001"]);
        assert_eq!(titles(&without), ["Open linker"], "{query}");
        let mut with = titles(&read_json(&bundle, &[query, "adr/0001", "--all"]));
        with.sort();
        assert_eq!(with, ["Closed linker", "Open linker"], "{query}");
    }
}

#[test]
fn plain_text_names_the_withheld_count_for_closed_and_done_records() {
    let bundle = temp_bundle("read-retired", "text");
    write_bundle(&bundle);

    let output = read(&bundle, &["--plain"]);

    let text = stdout_of(&output);
    assert!(text.contains("Withheld 5 retired record(s)"), "{text}");
    assert!(text.contains("closed"), "{text}");
    assert!(text.contains("done"), "{text}");
}

#[test]
fn an_unmatched_topic_under_all_does_not_call_the_records_live() {
    let bundle = temp_bundle("read-retired", "hint");
    write_bundle(&bundle);

    let output = read(&bundle, &["--topic", "zzqq", "--all", "--plain"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no record mentions 'zzqq'"), "{stderr}");
    assert!(!stderr.contains("live"), "{stderr}");
}
