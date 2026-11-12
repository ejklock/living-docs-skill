use living_docs_mcp::BundleServer;
use rmcp::model::{CallToolRequestParams, CallToolResult, ProtocolVersion};
use rmcp::{ClientHandler, ClientLifecycleMode, ClientServiceExt, RoleClient, ServiceExt};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Once;

pub const ADR: &str = "adr/0001-use-postgres.md";
pub const BODY: &str = "## Context\n\nThe forces at play.\n\n## Decision\n\nWe will use it.\n\n## Consequences\n\n**Easier / gained:**\n- Speed\n\n**Harder / accepted trade-offs:**\n- Cost\n\n## Verification\n\n**Implementation impact:** the store.\n\n**Verification criteria:**\n- A test fails when violated.\n";

#[derive(Clone, Default)]
pub struct Probe;

impl ClientHandler for Probe {}

pub type Client = rmcp::service::RunningService<RoleClient, Probe>;

pub enum Era {
    Initialize,
    Discover,
}

/// A scratch directory holding the served bundle and a sibling directory the
/// server must never touch.
pub struct Scratch {
    root: PathBuf,
}

static NEXT: AtomicU32 = AtomicU32::new(0);
static PIN_GIT: Once = Once::new();

/// Hides the machine's global and system git config from every test in this
/// binary, so an identity comes only from a repository a test builds.
fn pin_git_environment() {
    PIN_GIT.call_once(|| {
        std::env::set_var("GIT_CONFIG_GLOBAL", "/dev/null");
        std::env::set_var("GIT_CONFIG_SYSTEM", "/dev/null");
        std::env::set_var("GIT_CONFIG_NOSYSTEM", "1");
    });
}

impl Scratch {
    pub fn new() -> Self {
        pin_git_environment();
        let id = NEXT.fetch_add(1, Ordering::SeqCst);
        let root =
            std::env::temp_dir().join(format!("living-docs-mcp-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("bundle")).expect("scratch bundle");
        fs::create_dir_all(root.join("outside")).expect("scratch outside");
        Self { root }
    }

    pub fn bundle(&self) -> PathBuf {
        self.root.join("bundle")
    }

    pub fn outside(&self) -> PathBuf {
        self.root.join("outside")
    }

    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.bundle().join(relative)).expect("bundle file")
    }

    /// Makes the bundle a git repository whose local config holds `config`.
    pub fn init_repo(&self, config: &[(&str, &str)]) {
        git(&self.bundle(), &["init", "-q"], None);
        for (key, value) in config {
            git(&self.bundle(), &["config", key, value], None);
        }
    }

    /// Commits everything in the bundle with `date` as the author date.
    pub fn commit_all(&self, date: &str) {
        git(&self.bundle(), &["add", "."], None);
        let identity = [
            "-c",
            "user.name=Committer",
            "-c",
            "user.email=c@example.com",
        ];
        let args = [&identity[..], &["commit", "-q", "-m", "add"]].concat();
        git(&self.bundle(), &args, Some(date));
    }

    /// Drops every line starting with `prefix` from a bundle file.
    pub fn strip_line(&self, relative: &str, prefix: &str) {
        let kept: Vec<String> = self
            .read(relative)
            .lines()
            .filter(|line| !line.starts_with(prefix))
            .map(|line| format!("{line}\n"))
            .collect();
        fs::write(self.bundle().join(relative), kept.concat()).expect("rewrite file");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(dir: &Path, args: &[&str], date: Option<&str>) {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).args(args);
    if let Some(date) = date {
        cmd.env("GIT_AUTHOR_DATE", date);
    }
    let status = cmd.status().expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

/// Every file under `dir` with its contents, so a test can prove nothing changed.
pub fn snapshot(dir: &Path) -> BTreeMap<PathBuf, String> {
    let mut found = BTreeMap::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            found.extend(snapshot(&path));
        } else {
            found.insert(path.clone(), fs::read_to_string(&path).unwrap_or_default());
        }
    }
    found
}

pub async fn connect(bundle: &Path, era: Era) -> Client {
    pin_git_environment();
    let (server_io, client_io) = tokio::io::duplex(1 << 16);
    let server = BundleServer::new(bundle);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    match era {
        Era::Initialize => Probe.serve(client_io).await.expect("initialize handshake"),
        Era::Discover => Probe
            .serve_with_lifecycle(
                client_io,
                ClientLifecycleMode::Discover {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                },
            )
            .await
            .expect("server/discover startup"),
    }
}

pub async fn call(client: &Client, tool: &'static str, arguments: Value) -> CallToolResult {
    let Value::Object(arguments) = arguments else {
        panic!("tool arguments are an object");
    };
    client
        .call_tool(CallToolRequestParams::new(tool).with_arguments(arguments))
        .await
        .expect("the server answers the call")
}

pub fn is_error(result: &CallToolResult) -> bool {
    result.is_error == Some(true)
}

pub fn data(result: &CallToolResult) -> Value {
    result
        .structured_content
        .clone()
        .unwrap_or_else(|| panic!("no structured content in {result:?}"))
}

pub fn message(result: &CallToolResult) -> String {
    serde_json::to_value(&result.content)
        .expect("content serializes")
        .pointer("/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub async fn new_adr(client: &Client, title: &str, body: &str) -> Value {
    let result = call(
        client,
        "write",
        json!({
            "operation": "new", "type": "adr", "title": title,
            "description": "A decision.", "owner": "Eve", "body": body
        }),
    )
    .await;
    assert!(!is_error(&result), "new adr failed: {}", message(&result));
    data(&result)
}

pub async fn adr_after_seeding(config: &[(&str, &str)]) -> String {
    let scratch = Scratch::new();
    if !config.is_empty() {
        scratch.init_repo(config);
    }
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let result = new_adr_without_owner(&client, "Use postgres").await;
    assert!(!is_error(&result), "{}", message(&result));
    scratch.read(ADR)
}

/// Writes an adr, strips `prefix` from it, optionally commits it in a repo
/// with `config`, then runs `manage fmt`.
pub async fn fmt_after_stripping(prefix: &str, config: &[(&str, &str)]) -> Scratch {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&client, "Use postgres", BODY).await;
    scratch.strip_line(ADR, prefix);
    if !config.is_empty() {
        scratch.init_repo(config);
        scratch.commit_all("2020-01-02T05:04:05+0200");
    }
    let result = call(&client, "manage", json!({"operation": "fmt"})).await;
    assert!(!is_error(&result), "{}", message(&result));
    scratch
}

/// Calls `write new` for an adr with no owner argument.
pub async fn new_adr_without_owner(client: &Client, title: &str) -> CallToolResult {
    let args = json!({
        "operation": "new", "type": "adr", "title": title,
        "description": "A decision.", "body": BODY
    });
    call(client, "write", args).await
}
