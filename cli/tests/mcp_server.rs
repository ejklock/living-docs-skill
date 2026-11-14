//! `living-docs mcp` over a real child process: newline-delimited JSON-RPC on
//! stdin and stdout, nothing else on stdout, and a refusal to start on a
//! missing bundle root.

mod common;

use common::mcp_session::Session;
use serde_json::{json, Value};
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
fn refuses_a_missing_bundle_root_before_serving() {
    let scratch = common::Scratch::new("living-docs-mcp-missing");
    let missing = scratch.join("nowhere");
    let output = common::living_docs()
        .args(["mcp", "--docs-dir", missing.to_str().unwrap()])
        .stdin(Stdio::null())
        .output()
        .expect("failed to run living-docs mcp");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "stdout: {:?}", output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 1, "stderr: {stderr}");
    assert!(
        stderr.starts_with("living-docs mcp: ") && stderr.contains(missing.to_str().unwrap()),
        "stderr: {stderr}"
    );
}
