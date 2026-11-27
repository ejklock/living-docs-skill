//! A server pointed at a bundle path that does not exist yet: it still lists
//! its tools, answers a bundle-less call the way the CLI does, and `write new`
//! creates the bundle for the calls that follow.

use crate::common::{call, connect, data, is_error, message, new_adr, Era, Scratch, BODY};
use serde_json::json;
use std::fs;

#[tokio::test]
async fn a_missing_bundle_lists_tools_and_check_names_the_path() {
    let scratch = Scratch::new();
    let missing = scratch.outside().join("nowhere");
    let client = connect(&missing, Era::Initialize).await;

    assert_eq!(client.list_all_tools().await.expect("tools/list").len(), 4);
    let result = call(&client, "check", json!({})).await;
    assert!(is_error(&result), "{result:?}");
    assert!(
        message(&result).contains(missing.to_str().unwrap()),
        "{}",
        message(&result)
    );
    assert!(!missing.exists(), "a read-only call must not create it");
}

#[tokio::test]
async fn a_missing_bundle_reads_zero_records_in_force_as_the_cli_does() {
    let scratch = Scratch::new();
    let missing = scratch.outside().join("nowhere");
    let client = connect(&missing, Era::Initialize).await;

    let result = call(&client, "read", json!({"operation": "records"})).await;
    assert!(!is_error(&result), "{}", message(&result));
    let found = data(&result);
    assert_eq!(found["records"], json!([]));
    assert_eq!(found["withheld"], json!(0));
    assert!(!missing.exists(), "a read-only call must not create it");
}

#[tokio::test]
async fn write_new_creates_the_missing_bundle_and_later_calls_serve_it() {
    let scratch = Scratch::new();
    let missing = scratch.outside().join("nowhere");
    let client = connect(&missing, Era::Initialize).await;

    new_adr(&client, "Use postgres", BODY).await;
    let root = fs::read_to_string(missing.join("index.md")).expect("root index");
    assert!(root.contains("okf_version:"), "{root}");

    let listed = data(&call(&client, "read", json!({"operation": "records"})).await);
    let records = listed["records"].as_array().expect("records");
    assert_eq!(records.len(), 1, "{listed}");
    assert_eq!(records[0]["title"], json!("Use postgres"), "{listed}");

    let checked = call(&client, "check", json!({})).await;
    assert!(
        !message(&checked).contains("bundle root not found"),
        "{}",
        message(&checked)
    );
    assert!(!is_error(&checked), "{}", message(&checked));
}

#[tokio::test]
async fn write_new_creates_every_missing_parent_of_the_bundle_path() {
    let scratch = Scratch::new();
    let missing = scratch.outside().join("a").join("b").join("docs");
    let client = connect(&missing, Era::Initialize).await;

    new_adr(&client, "Use postgres", BODY).await;
    assert!(missing.join("index.md").is_file());
    let result = call(&client, "read", json!({"operation": "records"})).await;
    assert!(!is_error(&result), "{}", message(&result));
}

#[tokio::test]
async fn a_bundle_path_that_is_a_regular_file_is_a_tool_error_and_the_server_keeps_answering() {
    let scratch = Scratch::new();
    let file = scratch.outside().join("plain-file");
    fs::write(&file, "not a bundle").expect("plain file");
    let client = connect(&file, Era::Initialize).await;

    let result = call(&client, "check", json!({})).await;
    assert!(is_error(&result), "{result:?}");
    assert!(
        message(&result).contains(file.to_str().unwrap()),
        "{}",
        message(&result)
    );
    assert_eq!(client.list_all_tools().await.expect("tools/list").len(), 4);
    assert_eq!(
        fs::read_to_string(&file).expect("file intact"),
        "not a bundle"
    );
}
