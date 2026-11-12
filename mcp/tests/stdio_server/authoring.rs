use crate::common::{
    call, connect, data, is_error, message, new_adr, snapshot, Era, Scratch, BODY,
};
use serde_json::json;

#[tokio::test]
async fn write_then_check_new_adr_leaves_a_conformant_bundle() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let created = new_adr(&client, "Use postgres", BODY).await;
    assert!(created["path"]
        .as_str()
        .unwrap()
        .ends_with("adr/0001-use-postgres.md"));
    assert!(created["instruction"].is_string());
    assert!(created["indexed"].is_array());
    let report = data(&call(&client, "check", json!({})).await);
    assert_eq!(report["ok"], true, "{report}");
    assert!(scratch
        .read("adr/index.md")
        .contains("0001-use-postgres.md"));
}

#[tokio::test]
async fn write_then_check_set_changes_the_field() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    let args = json!({
        "operation": "set", "reference": "adr/0001", "key": "status", "value": "Accepted"
    });
    let applied = call(&client, "write", args).await;
    assert!(!is_error(&applied), "{}", message(&applied));
    assert_eq!(data(&applied)["key"], "status");
    assert!(scratch
        .read("adr/0001-use-postgres.md")
        .contains("status: Accepted"));
}

#[tokio::test]
async fn write_then_check_set_section_replaces_only_that_section() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    let args = json!({
        "operation": "set", "reference": "adr/0001",
        "heading": "Context", "text": "Fresh context."
    });
    let applied = call(&client, "write", args).await;
    assert!(!is_error(&applied), "{}", message(&applied));
    let record = scratch.read("adr/0001-use-postgres.md");
    assert!(record.contains("Fresh context."));
    assert!(!record.contains("The forces at play."));
    assert!(record.contains("We will use it."));
}

#[tokio::test]
async fn write_then_check_supersede_links_both_records() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    new_adr(&client, "Use sqlite", BODY).await;
    let args = json!({"operation": "supersede", "old": "adr/0001", "new": "adr/0002"});
    let result = call(&client, "write", args).await;
    assert!(!is_error(&result), "{}", message(&result));
    let old = scratch.read("adr/0001-use-postgres.md");
    let new = scratch.read("adr/0002-use-sqlite.md");
    assert!(old.contains("status: Superseded"), "{old}");
    assert!(old.contains("0002-use-sqlite.md"), "{old}");
    assert!(new.contains("supersedes: 0001"), "{new}");
}

#[tokio::test]
async fn manage_index_and_fmt_report_the_files_they_touch() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    let index = call(
        &client,
        "manage",
        json!({"operation": "index", "type": "adr"}),
    )
    .await;
    assert!(!is_error(&index), "{}", message(&index));
    assert!(data(&index)["files"].is_array());
    let fmt = call(
        &client,
        "manage",
        json!({"operation": "fmt", "check_only": true}),
    )
    .await;
    assert!(!is_error(&fmt), "{}", message(&fmt));
    assert_eq!(data(&fmt)["pending"], json!([]));
}

#[tokio::test]
async fn write_then_check_unknown_type_is_refused_with_cores_message() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let before = snapshot(&scratch.bundle());
    let args = json!({"operation": "new", "type": "nonsense", "title": "x"});
    let result = call(&client, "write", args).await;
    assert!(is_error(&result));
    assert!(message(&result).contains("unsupported doc type 'nonsense'"));
    assert_eq!(snapshot(&scratch.bundle()), before);
}

#[tokio::test]
async fn write_then_check_set_without_a_value_or_text_is_refused() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    let before = snapshot(&scratch.bundle());
    let args = json!({"operation": "set", "reference": "adr/0001"});
    let result = call(&client, "write", args).await;
    assert!(is_error(&result), "{}", message(&result));
    assert_eq!(snapshot(&scratch.bundle()), before);
}

fn set_index_writable(scratch: &Scratch, writable: bool) {
    let path = scratch.bundle().join("adr/index.md");
    let mut permissions = std::fs::metadata(&path)
        .expect("index exists")
        .permissions();
    permissions.set_readonly(!writable);
    std::fs::set_permissions(&path, permissions).expect("index permissions");
}

/// A failed refresh still lists the written indexes and names each failure.
fn assert_refresh_failed(reported: &serde_json::Value) {
    assert!(reported["indexed"].is_array(), "{reported}");
    let failures = reported["failures"].as_array().expect("failures array");
    assert!(
        failures
            .iter()
            .any(|failure| failure.as_str().is_some_and(|text| {
                text.contains("index not updated") && text.contains("manage index adr")
            })),
        "{reported}"
    );
}

#[tokio::test]
async fn index_refresh_failure_after_new_still_reports_the_written_record() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    set_index_writable(&scratch, false);
    let args = json!({
        "operation": "new", "type": "adr", "title": "Use sqlite",
        "description": "A decision.", "owner": "Eve", "body": BODY
    });
    let result = call(&client, "write", args).await;
    set_index_writable(&scratch, true);
    assert!(is_error(&result), "{result:?}");
    let reported = data(&result);
    assert!(reported["path"]
        .as_str()
        .unwrap()
        .ends_with("adr/0002-use-sqlite.md"));
    assert!(reported["instruction"].is_string(), "{reported}");
    assert_refresh_failed(&reported);
    assert!(scratch
        .read("adr/0002-use-sqlite.md")
        .contains("Use sqlite"));
}

#[tokio::test]
async fn index_refresh_failure_after_set_still_reports_the_written_record() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    set_index_writable(&scratch, false);
    let args = json!({
        "operation": "set", "reference": "adr/0001", "key": "status", "value": "Accepted"
    });
    let result = call(&client, "write", args).await;
    set_index_writable(&scratch, true);
    assert!(is_error(&result), "{result:?}");
    let reported = data(&result);
    assert!(reported["path"]
        .as_str()
        .unwrap()
        .ends_with("adr/0001-use-postgres.md"));
    for field in ["key", "value", "rewritten"] {
        assert!(reported.get(field).is_some(), "{field} missing: {reported}");
    }
    assert_refresh_failed(&reported);
    assert!(scratch
        .read("adr/0001-use-postgres.md")
        .contains("status: Accepted"));
}

#[tokio::test]
async fn index_refresh_success_lists_the_index_and_is_not_an_error() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let created = new_adr(&client, "Use postgres", BODY).await;
    let listed = created["indexed"].as_array().unwrap();
    assert!(listed
        .iter()
        .any(|file| file.as_str().unwrap().ends_with("adr/index.md")));
    assert!(created.get("failures").is_none(), "{created}");
}
