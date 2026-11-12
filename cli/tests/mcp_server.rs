//! `living-docs mcp` over a real child process: newline-delimited JSON-RPC on
//! stdin and stdout, nothing else on stdout, and a refusal to start on a
//! missing bundle root.

mod common;

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

const ANSWER_DEADLINE: Duration = Duration::from_secs(20);
const EXIT_DEADLINE: Duration = Duration::from_secs(10);

/// A child `living-docs mcp` whose stdout is read on a helper thread, so every
/// wait on the server has a deadline; the child is always killed and reaped
/// when the session ends, including when an assertion panics.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    seen: Vec<String>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Session {
    fn start(docs_dir: &std::path::Path) -> Session {
        let mut child = common::living_docs()
            .args(["mcp", "--docs-dir", docs_dir.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to spawn living-docs mcp");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Session {
            child,
            stdin,
            lines,
            seen: Vec::new(),
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    fn request(&mut self, id: u64, method: &str, params: &Value) -> Value {
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let line = match self.lines.recv_timeout(ANSWER_DEADLINE) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => {
                    panic!("no answer to {method} within {ANSWER_DEADLINE:?}")
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("stdout closed before the answer to {method}")
                }
            };
            self.seen.push(line.clone());
            let message: Value = serde_json::from_str(line.trim())
                .unwrap_or_else(|_| panic!("stdout line is not JSON: {line:?}"));
            if message["id"] == json!(id) {
                return message;
            }
        }
    }

    fn initialize(&mut self) -> Value {
        let answer = self.request(
            1,
            "initialize",
            &json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0"}
            }),
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        answer
    }

    fn finish(mut self) -> (Option<i32>, Vec<String>) {
        drop(self.stdin.take());
        let deadline = Instant::now() + EXIT_DEADLINE;
        while Instant::now() < deadline {
            if let Some(status) = self.child.try_wait().unwrap() {
                while let Ok(line) = self.lines.recv_timeout(EXIT_DEADLINE) {
                    self.seen.push(line);
                }
                return (status.code(), std::mem::take(&mut self.seen));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        (None, std::mem::take(&mut self.seen))
    }
}

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
