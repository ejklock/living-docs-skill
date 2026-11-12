//! Runs one operation through `living-docs <verb> --json` and through the
//! MCP tool, each on its own copy of one fixture bundle, and reports every
//! difference in the files left behind or the JSON returned.

use crate::common::mcp_session::Session;
use crate::common::parity_bundles::{make_bundle, set_index_writable, Bundle, BODY};
use crate::common::{copy_tree, living_docs, Scratch};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Case {
    name: String,
    bundle: Bundle,
    cli: Vec<String>,
    tool: String,
    args: Value,
    fails: bool,
    partial: bool,
    report: bool,
    shows: Vec<String>,
}

/// Reads table rows of `name | cli words | tool | tool arguments as JSON |
/// flags`. The flags are optional and comma-separated: `canonical`,
/// `noncanonical`, `stale` or `readonly` (the decision index is read-only)
/// pick the fixture bundle, `fails` expects both paths to refuse the call
/// and change no file, `partial` expects both to write the record and fail
/// the index refresh, `report` expects the CLI to exit non-zero while the
/// tool returns its result without an error, `shows=<fragment>` requires that
/// compact JSON fragment in both results. In `{body}`, a CLI word is the body file's path and a tool
/// argument is its text.
pub fn parse(table: &[&str]) -> Vec<Case> {
    table.iter().map(|row| parse_row(row)).collect()
}

fn parse_row(row: &str) -> Case {
    let mut cells = row.split(" | ");
    let mut next = || cells.next().unwrap_or_else(|| panic!("short row: {row}"));
    let (name, cli, tool, args) = (next(), next(), next(), next());
    let flags: Vec<&str> = cells
        .next()
        .map_or_else(Vec::new, |f| f.split(',').collect());
    let bundle = [
        ("canonical", Bundle::Canonical),
        ("noncanonical", Bundle::NonCanonical),
        ("stale", Bundle::StaleIndexes),
        ("readonly", Bundle::ReadOnlyIndex),
    ]
    .into_iter()
    .find(|(flag, _)| flags.contains(flag))
    .map_or(Bundle::Scaffolded, |(_, bundle)| bundle);
    Case {
        name: name.to_string(),
        bundle,
        cli: cli.split_whitespace().map(str::to_string).collect(),
        tool: tool.to_string(),
        args: serde_json::from_str(args).unwrap_or_else(|e| panic!("{name}: {e}")),
        fails: flags.contains(&"fails"),
        partial: flags.contains(&"partial"),
        report: flags.contains(&"report"),
        shows: flags
            .iter()
            .filter_map(|f| f.strip_prefix("shows="))
            .map(str::to_string)
            .collect(),
    }
}

type Files = BTreeMap<String, String>;

fn snapshot(root: &Path, dir: &Path, into: &mut Files) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            snapshot(root, &path, into);
        } else {
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            into.insert(rel, fs::read_to_string(&path).unwrap());
        }
    }
}

fn files_of(root: &Path) -> Files {
    let mut files = Files::new();
    snapshot(root, root, &mut files);
    files
}

/// Waits for a fresh second when the current one is nearly spent, so both
/// paths stamp a new record inside the same second.
fn align_to_second() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let spent = Duration::from_nanos(u64::from(nanos));
    if spent > Duration::from_millis(400) {
        std::thread::sleep(Duration::from_secs(1) - spent);
    }
}

/// Names a copy's root, in its canonical form first because the plain form of
/// a temp directory is a suffix of it, so one placeholder stands for both.
fn normalize(value: &Value, docs: &Path) -> Value {
    let canonical = docs.canonicalize().unwrap();
    let mut text = value.to_string();
    for root in [canonical.display().to_string(), docs.display().to_string()] {
        text = text.replace(root.as_str(), "<root>");
    }
    serde_json::from_str(&text).unwrap()
}

struct Outcome {
    cli_ok: bool,
    cli: Option<Value>,
    cli_err: String,
    mcp_error: bool,
    mcp: Option<Value>,
}

/// Swaps each `{body}` argument for the text of the body file, which the CLI
/// reads from a path and the tool takes inline.
fn inline_body(args: &Value, body: &Path) -> Value {
    let mut args = args.clone();
    for key in ["body", "text"] {
        if args[key] == "{body}" {
            args[key] = json!(fs::read_to_string(body).unwrap());
        }
    }
    args
}

fn run_cli(case: &Case, docs: &Path, body: &Path) -> std::process::Output {
    let body = body.to_str().unwrap();
    let words = case
        .cli
        .iter()
        .map(|word| if word == "{body}" { body } else { word });
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap()])
        .args(words)
        .arg("--json")
        .output()
        .unwrap()
}

fn run_both(case: &Case, cli_docs: &Path, mcp_docs: &Path, body: &Path) -> Outcome {
    let mut session = Session::start(mcp_docs);
    session.initialize();
    align_to_second();
    let output = run_cli(case, cli_docs, body);
    let result = session.call(2, &case.tool, &inline_body(&case.args, body));
    Outcome {
        cli_ok: output.status.success(),
        cli_err: String::from_utf8_lossy(&output.stderr).into_owned(),
        cli: serde_json::from_slice(&output.stdout).ok(),
        mcp_error: result["isError"] == json!(true),
        mcp: result.get("structuredContent").cloned().or(Some(result)),
    }
}

fn failed_problem(outcome: &Outcome) -> Option<String> {
    (outcome.cli_ok || !outcome.mcp_error).then(|| {
        format!(
            "expected both to fail: cli ok {}, mcp error {}",
            outcome.cli_ok, outcome.mcp_error
        )
    })
}

fn refusal_problems(outcome: &Outcome, files: [&Files; 3]) -> Vec<String> {
    let mut problems: Vec<String> = failed_problem(outcome).into_iter().collect();
    let [start, cli, mcp] = files;
    if cli != start || mcp != start {
        problems.push("a failed call changed files".to_string());
    }
    problems
}

/// Each failure the tool names must appear in the CLI's stderr.
fn failure_problems(outcome: &Outcome) -> Vec<String> {
    let failures = outcome
        .mcp
        .as_ref()
        .and_then(|mcp| mcp["failures"].as_array());
    let Some(failures) = failures.filter(|list| !list.is_empty()) else {
        return vec!["the tool reported no failures".to_string()];
    };
    let stderr = &outcome.cli_err;
    failures
        .iter()
        .map(|failure| failure.as_str().unwrap_or_default().to_string())
        .filter(|text| text.is_empty() || !stderr.contains(text.as_str()))
        .map(|text| format!("cli stderr lacks failure {text:?}:\n{stderr}"))
        .collect()
}

/// A write whose index refresh failed keeps the new record and the old index
/// in both copies, with the same JSON apart from the tool's `failures`.
fn partial_problems(outcome: &Outcome, docs: [&Path; 2], files: [&Files; 3]) -> Vec<String> {
    let [start, cli_files, mcp_files] = files;
    let mut problems = failure_problems(outcome);
    problems.extend(failed_problem(outcome));
    let mut mcp = outcome.mcp.clone();
    if let Some(Value::Object(map)) = mcp.as_mut() {
        map.remove("failures");
    }
    let cli = outcome.cli.as_ref().map(|v| normalize(v, docs[0]));
    let mcp = mcp.as_ref().map(|v| normalize(v, docs[1]));
    if cli != mcp || cli.is_none() {
        problems.push(format!("json differs\n  cli: {cli:?}\n  mcp: {mcp:?}"));
    }
    let kept = start
        .iter()
        .all(|(path, text)| cli_files.get(path) == Some(text));
    if cli_files != mcp_files || !kept || cli_files.len() <= start.len() {
        problems.push(format!(
            "files differ\n  cli: {cli_files:#?}\n  mcp: {mcp_files:#?}"
        ));
    }
    problems
}

fn result_problems(case: &Case, cli: &Option<Value>, mcp: &Option<Value>) -> Vec<String> {
    let has_all = |json: &Option<Value>| {
        let text = json.as_ref().map(Value::to_string).unwrap_or_default();
        case.shows.iter().all(|shown| text.contains(shown.as_str()))
    };
    let mut problems = Vec::new();
    if !has_all(cli) || !has_all(mcp) {
        problems.push(format!("a result lacks {:?}", case.shows));
    }
    if cli != mcp || cli.is_none() {
        problems.push("json differs".to_string());
    }
    problems
        .into_iter()
        .map(|problem| format!("{problem}\n  cli: {cli:?}\n  mcp: {mcp:?}"))
        .collect()
}

fn status_problems(case: &Case, outcome: &Outcome) -> Option<String> {
    let expected_cli_ok = !case.report;
    (outcome.cli_ok != expected_cli_ok || outcome.mcp_error).then(|| {
        format!(
            "status differs: cli ok {} (expected {expected_cli_ok}), mcp error {} (expected false)",
            outcome.cli_ok, outcome.mcp_error
        )
    })
}

fn compare(case: &Case, outcome: &Outcome, docs: [&Path; 2], start: &Files) -> Vec<String> {
    let (cli_files, mcp_files) = (files_of(docs[0]), files_of(docs[1]));
    if case.fails {
        return refusal_problems(outcome, [start, &cli_files, &mcp_files]);
    }
    if case.partial {
        return partial_problems(outcome, docs, [start, &cli_files, &mcp_files]);
    }
    let cli = outcome.cli.as_ref().map(|v| normalize(v, docs[0]));
    let mcp = outcome.mcp.as_ref().map(|v| normalize(v, docs[1]));
    let mut problems = result_problems(case, &cli, &mcp);
    problems.extend(status_problems(case, outcome));
    if cli_files != mcp_files {
        problems.push(format!(
            "files differ\n  cli: {cli_files:#?}\n  mcp: {mcp_files:#?}"
        ));
    }
    problems
}

fn mismatches(case: &Case) -> Vec<String> {
    let cli = Scratch::with_subdir("mcp-parity-cli", "docs");
    let mcp = Scratch::with_subdir("mcp-parity-mcp", "docs");
    let body = cli.root().join("body.md");
    fs::write(&body, BODY).unwrap();
    make_bundle(case.bundle, &cli, &body);
    copy_tree(&cli, &mcp);
    let start = files_of(&cli);
    let lock = |writable| {
        if matches!(case.bundle, Bundle::ReadOnlyIndex) {
            set_index_writable(&[&cli, &mcp], writable);
        }
    };
    lock(false);
    let outcome = run_both(case, &cli, &mcp, &body);
    lock(true);
    compare(case, &outcome, [&cli, &mcp], &start)
        .into_iter()
        .map(|problem| format!("{}: {problem}", case.name))
        .collect()
}

pub fn assert_parity(cases: &[Case]) {
    let problems: Vec<String> = cases.iter().flat_map(mismatches).collect();
    assert!(problems.is_empty(), "\n{}", problems.join("\n"));
}
