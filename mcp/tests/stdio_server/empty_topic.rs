//! The `read` operation `records` with a topic no record mentions: the answer
//! carries the in-force count and a note, and a matching topic stays as it was.

use crate::common::{call, connect, data, is_error, message, Era, Scratch};
use serde_json::{json, Value};
use std::fs;

fn adr(number: &str, status: &str, extra: &str) -> String {
    format!(
        "---\ntype: ADR\ntitle: Rule {number}\ndescription: d\nstatus: {status}\n{extra}---\n\n# {number}. Rule {number}\n\nabout widgets\n"
    )
}

fn bundle() -> Scratch {
    let scratch = Scratch::new();
    let dir = scratch.bundle().join("adr");
    fs::create_dir_all(&dir).expect("adr dir");
    for (file, number, status, extra) in [
        ("0001-a.md", "0001", "Accepted", ""),
        ("0002-b.md", "0002", "Accepted", ""),
        ("0003-c.md", "0003", "Accepted", ""),
        (
            "0004-d.md",
            "0004",
            "Superseded",
            "superseded_by: \"0003\"\n",
        ),
    ] {
        fs::write(dir.join(file), adr(number, status, extra)).expect("adr record");
    }
    scratch
}

async fn topic(scratch: &Scratch, topic: &str) -> Value {
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let args = json!({"operation": "records", "topic": topic});
    let result = call(&client, "read", args).await;
    assert!(!is_error(&result), "{}", message(&result));
    data(&result)
}

#[tokio::test]
async fn an_empty_topic_carries_the_in_force_count_and_a_note() {
    let scratch = bundle();

    let answer = topic(&scratch, "zzqq").await;

    assert_eq!(answer["in_force"], 3);
    assert_eq!(answer["withheld"], 1);
    assert_eq!(answer["records"], json!([]));
    let note = answer["note"].as_str().expect("note string");
    assert!(note.contains("3 records in force"), "{note}");
    assert!(note.contains("not a conflict check"), "{note}");
    assert!(note.contains("`living-docs read records`"), "{note}");
}

#[tokio::test]
async fn the_note_is_the_one_the_core_writes_for_the_same_bundle() {
    let scratch = bundle();

    let answer = topic(&scratch, "zzqq").await;

    let expected =
        living_docs_core::commands::effective::empty_topic_note(3, "zzqq", &scratch.bundle());
    assert_eq!(answer["note"], expected);
}

#[tokio::test]
async fn an_empty_topic_under_all_still_counts_only_the_records_in_force() {
    let scratch = bundle();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let args = json!({"operation": "records", "topic": "zzqq", "all": true});

    let result = call(&client, "read", args).await;

    let answer = data(&result);
    assert_eq!(answer["in_force"], 3);
    assert_eq!(answer["withheld"], 0);
}

#[tokio::test]
async fn a_matching_topic_has_no_count_or_note() {
    let scratch = bundle();

    let answer = topic(&scratch, "widget").await;

    assert!(answer.get("in_force").is_none(), "{answer}");
    assert!(answer.get("note").is_none(), "{answer}");
    assert_eq!(answer["records"].as_array().map(Vec::len), Some(3));
}
