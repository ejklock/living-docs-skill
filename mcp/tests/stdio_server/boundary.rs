use crate::common::{call, connect, is_error, message, new_adr, snapshot, Era, Scratch, BODY};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::symlink;

const SECRET: &str = "---\ntype: ADR\ntitle: Secret\nstatus: Proposed\n---\n\n# 0001. Secret\n";

async fn refused(scratch: &Scratch, args: Value) {
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let before = snapshot(&scratch.outside());
    let result = call(&client, "write", args.clone()).await;
    assert!(
        is_error(&result),
        "{args} was accepted: {}",
        message(&result)
    );
    assert_eq!(
        snapshot(&scratch.outside()),
        before,
        "{args} changed the outside"
    );
}

#[tokio::test]
async fn refuses_outside_the_bundle_a_reference_with_parent_segments() {
    let scratch = Scratch::new();
    fs::write(scratch.outside().join("0001-secret.md"), SECRET).unwrap();
    let reference = "../outside/0001-secret.md";
    let args =
        json!({"operation": "set", "reference": reference, "key": "status", "value": "Accepted"});
    refused(&scratch, args).await;
}

#[tokio::test]
async fn refuses_outside_the_bundle_a_symlinked_directory() {
    let scratch = Scratch::new();
    fs::write(scratch.outside().join("0001-secret.md"), SECRET).unwrap();
    symlink(scratch.outside(), scratch.bundle().join("adr")).unwrap();
    let set =
        json!({"operation": "set", "reference": "adr/0001", "key": "status", "value": "Accepted"});
    refused(&scratch, set).await;
    let new = json!({"operation": "new", "type": "adr", "title": "Escape", "owner": "Eve"});
    refused(&scratch, new).await;
}

#[tokio::test]
async fn refuses_outside_the_bundle_a_malformed_reference() {
    let scratch = Scratch::new();
    for reference in ["adr/", "adr/abc"] {
        let args = json!({"operation": "set", "reference": reference, "key": "status", "value": "Accepted"});
        refused(&scratch, args).await;
        let args = json!({"operation": "supersede", "old": reference, "new": "adr/0001"});
        refused(&scratch, args).await;
    }
}

#[tokio::test]
async fn refuses_outside_the_bundle_a_title_with_parent_segments_stays_inside() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let before = snapshot(&scratch.outside());
    let created = new_adr(&client, "../../outside/evil", BODY).await;
    let path = created["path"].as_str().unwrap();
    assert!(
        path.starts_with(scratch.bundle().to_str().unwrap()),
        "{path}"
    );
    assert_eq!(snapshot(&scratch.outside()), before);
}
