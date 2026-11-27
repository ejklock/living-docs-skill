//! `living-docs mcp` over a real child process: newline-delimited JSON-RPC on
//! stdin and stdout, nothing else on stdout, and a start on a missing bundle
//! root that creates nothing.

mod common;

use common::mcp_session::Session;
use serde_json::{json, Value};
use std::io::Write;
use std::process::Stdio;

#[test]
fn serves_the_four_tools_over_stdio_and_exits_zero_when_stdin_closes() {
    let bundle = common::fixture("09-okf-canonical");
    let mut session = Session::start(&bundle);

    let init = session.initialize();
    assert!(
        init["result"]["serverInfo"]["name"].is_string(),
        "got: {init}"
    );
    let listed = session.request(2, "tools/list", &json!({}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["write", "read", "manage", "check"]);
    let called = session.request(3, "tools/call", &json!({"name": "check", "arguments": {}}));
    assert_eq!(
        called["result"]["structuredContent"]["ok"],
        json!(true),
        "got: {called}"
    );

    let (code, seen) = session.finish();
    assert_eq!(code, Some(0), "the server must exit 0 once stdin closes");
    for line in seen {
        serde_json::from_str::<Value>(line.trim())
            .unwrap_or_else(|_| panic!("non-JSON-RPC stdout line: {line:?}"));
    }
}

#[test]
fn starts_and_lists_tools_on_a_missing_bundle_root_without_creating_it() {
    let scratch = common::Scratch::new("living-docs-mcp-missing");
    let missing = scratch.join("nowhere");
    let mut session = Session::start(&missing);

    let init = session.initialize();
    assert!(
        init["result"]["serverInfo"]["name"].is_string(),
        "got: {init}"
    );
    let listed = session.request(2, "tools/list", &json!({}));
    assert_eq!(
        listed["result"]["tools"].as_array().map(Vec::len),
        Some(4),
        "got: {listed}"
    );
    assert!(!missing.exists(), "starting must not create the bundle");

    let (code, seen) = session.finish();
    assert_eq!(code, Some(0), "the server must exit 0 once stdin closes");
    assert!(
        !missing.exists(),
        "listing tools must not create the bundle"
    );
    for line in seen {
        serde_json::from_str::<Value>(line.trim())
            .unwrap_or_else(|_| panic!("non-JSON-RPC stdout line: {line:?}"));
    }
}

#[test]
fn writes_nothing_to_stderr_while_serving_a_missing_bundle_root() {
    let scratch = common::Scratch::new("living-docs-mcp-missing-stderr");
    let missing = scratch.join("nowhere");
    let mut child = common::living_docs()
        .args(["mcp", "--docs-dir", missing.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn living-docs mcp");
    let requests = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-11-25", "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    ];
    let mut stdin = child.stdin.take().expect("stdin is piped");
    for request in requests {
        writeln!(stdin, "{request}").expect("send request");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("living-docs mcp exits");

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
    assert_eq!(String::from_utf8_lossy(&output.stdout).lines().count(), 2);
    assert!(!missing.exists());
}
