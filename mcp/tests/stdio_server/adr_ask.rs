//! The `write` operation `new` for an ADR asks whether the decision
//! contradicts a record in force; any other type answers as before.

use crate::common::{call, connect, data, is_error, message, Era, Scratch};
use living_docs_core::commands::effective::adr_ask;
use serde_json::{json, Value};
use std::fs;

fn bundle() -> Scratch {
    let scratch = Scratch::new();
    let dir = scratch.bundle().join("adr");
    fs::create_dir_all(&dir).expect("adr dir");
    for (number, status, extra) in [
        ("0001", "Accepted", ""),
        ("0002", "Accepted", ""),
        ("0003", "Proposed", ""),
        ("0004", "Superseded", "superseded_by: \"0003\"\n"),
        ("0005", "Deprecated", ""),
    ] {
        let text = format!(
            "---\ntype: ADR\ntitle: Rule {number}\ndescription: d\nstatus: {status}\n{extra}---\n\n# {number}. Rule {number}\n\nabout widgets\n"
        );
        fs::write(dir.join(format!("{number}-r.md")), text).expect("adr record");
    }
    scratch
}

async fn new_record(scratch: &Scratch, doc_type: &str) -> Value {
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let args = json!({"operation": "new", "type": doc_type, "title": "X", "owner": "Eve"});
    let result = call(&client, "write", args).await;
    assert!(!is_error(&result), "{}", message(&result));
    data(&result)
}

#[tokio::test]
async fn a_new_adr_returns_the_count_and_the_question_the_core_writes() {
    let scratch = bundle();

    let answer = new_record(&scratch, "adr").await;

    assert_eq!(answer["in_force"], 3);
    assert_eq!(answer["ask"], adr_ask());
    let keys: Vec<&String> = answer.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["path", "instruction", "indexed", "in_force", "ask"]);
}

#[tokio::test]
async fn a_new_adr_whose_index_refresh_fails_still_carries_the_count_and_question() {
    let scratch = bundle();
    fs::create_dir(scratch.bundle().join("adr/index.md")).expect("index as directory");
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let args = json!({"operation": "new", "type": "adr", "title": "X", "owner": "Eve"});

    let result = call(&client, "write", args).await;

    assert!(is_error(&result), "{result:?}");
    let answer = data(&result);
    let keys: Vec<&String> = answer.as_object().unwrap().keys().collect();
    let expected = [
        "path",
        "instruction",
        "indexed",
        "failures",
        "in_force",
        "ask",
    ];
    assert_eq!(keys, expected);
    assert_eq!(answer["in_force"], 3);
    assert_eq!(answer["ask"], adr_ask());
}

fn add_sharing_record(scratch: &Scratch) {
    let text = "---\ntype: ADR\ntitle: Shared key-value store for login state\ndescription: d\nstatus: Accepted\n---\n\n# 0006. Shared key-value store for login state\n\nabout widgets\n";
    fs::write(scratch.bundle().join("adr/0006-s.md"), text).expect("sharing record");
}

#[tokio::test]
async fn candidates_follow_the_question_on_success_and_on_the_index_refusal() {
    let scratch = bundle();
    add_sharing_record(&scratch);
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let args = json!({
        "operation": "new", "type": "adr", "title": "Sticky routing", "owner": "Eve",
        "body": "Route through the shared state store.\n",
    });

    let result = call(&client, "write", args.clone()).await;

    let answer = data(&result);
    assert_eq!(answer["candidates"][0]["number"], 6);
    assert_eq!(
        answer["candidates"][0]["shared"],
        json!(["shared", "state", "store"])
    );
    let keys: Vec<&String> = answer.as_object().unwrap().keys().collect();
    assert_eq!(keys.last().map(|k| k.as_str()), Some("candidates"));

    let refused = bundle();
    add_sharing_record(&refused);
    fs::create_dir(refused.bundle().join("adr/index.md")).expect("index as directory");
    let client = connect(&refused.bundle(), Era::Initialize).await;
    let result = call(&client, "write", args).await;
    assert!(is_error(&result), "{result:?}");
    assert_eq!(data(&result)["candidates"][0]["number"], 6);
}

#[tokio::test]
async fn a_new_issue_has_no_count_or_question() {
    let scratch = bundle();

    let answer = new_record(&scratch, "issue").await;

    assert!(answer.get("in_force").is_none(), "{answer}");
    assert!(answer.get("ask").is_none(), "{answer}");
}
