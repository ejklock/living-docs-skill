use crate::common::{call, connect, data, is_error, message, new_adr, Era, Scratch, BODY};
use serde_json::{json, Value};

#[tokio::test]
async fn check_reports_as_data_for_a_clean_bundle() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    let result = call(&client, "check", json!({})).await;
    assert!(!is_error(&result));
    let report = data(&result);
    assert_eq!(report["ok"], true);
    assert_eq!(report["violations"], json!([]));
}

#[tokio::test]
async fn check_reports_as_data_an_unfilled_slot_without_a_tool_error() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let args = json!({"operation": "new", "type": "adr", "title": "Use postgres", "owner": "Eve"});
    call(&client, "write", args).await;
    let result = call(&client, "check", json!({})).await;
    assert_eq!(result.is_error, Some(false));
    let report = data(&result);
    assert_eq!(report["ok"], false);
    let files: Vec<&str> = report["violations"]
        .as_array()
        .expect("violations")
        .iter()
        .filter_map(|violation| violation["file"].as_str())
        .collect();
    assert!(
        files
            .iter()
            .any(|file| file.ends_with("0001-use-postgres.md")),
        "{files:?}"
    );
}

fn numbers(result: &Value) -> Vec<i64> {
    result["records"]
        .as_array()
        .expect("records")
        .iter()
        .filter_map(|record| record["number"].as_i64())
        .collect()
}

async fn records(client: &crate::common::Client, extra: Value) -> Value {
    let mut args = json!({"operation": "records"});
    args.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    let result = call(client, "read", args).await;
    assert!(!is_error(&result), "{}", message(&result));
    data(&result)
}

#[tokio::test]
async fn read_records_filters_by_topic() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    new_adr(
        &client,
        "Pick a queue",
        &BODY.replace("The forces at play.", "Kafka throughput."),
    )
    .await;
    let found = records(&client, json!({"topic": "kafka"})).await;
    assert_eq!(numbers(&found), [2]);
}

#[tokio::test]
async fn read_records_withholds_a_superseded_record() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    new_adr(&client, "Use sqlite", BODY).await;
    let args = json!({"operation": "supersede", "old": "adr/0001", "new": "adr/0002"});
    call(&client, "write", args).await;
    let found = records(&client, json!({})).await;
    assert_eq!(numbers(&found), [2]);
    assert_eq!(found["withheld"], 1);
}

#[tokio::test]
async fn read_records_links_to_returns_the_records_that_link_it() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    let linking = BODY.replace(
        "We will use it.",
        "We will use it, per [the first](/adr/0001-use-postgres.md).",
    );
    new_adr(&client, "Follow up", &linking).await;
    new_adr(&client, "Unrelated", BODY).await;
    let found = records(&client, json!({"links_to": "adr/0001"})).await;
    assert_eq!(numbers(&found), [2]);
}
