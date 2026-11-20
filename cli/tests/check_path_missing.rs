//! The `path-missing` advisory: a live record naming, in inline code, a
//! repository path that does not exist. It never fails the gate and is
//! skipped when the bundle sits in no repository.

use serde_json::Value;
use std::fs;
use std::path::Path;

mod common;
use common::{living_docs, write};

fn record(dir: &str, extra: &str, body: &str) -> String {
    let (kind, sections) = if dir == "adr" {
        (
            "ADR",
            "## Context\n\nc.\n\n## Decision\n\nd.\n\n## Consequences\n\nq.\n",
        )
    } else {
        ("Issue", "## Scope\n\ns.\n\n## Acceptance\n\na.\n")
    };
    format!(
        "---\ntype: {kind}\ntitle: X\ndescription: A record.\nowner: a@b.c\n{extra}timestamp: 2026-11-20T10:00:00Z\n---\n\n# 0001. X\n\n{body}\n\n{sections}"
    )
}

fn adr(body: &str) -> String {
    record("adr", "status: Accepted\n", body)
}

/// A scratch repository (a `.git` directory, `src/lib.rs`) holding a bundle
/// at `docs` with the given records, indexed.
fn repository(label: &str, records: &[(&str, String)], with_git: bool) -> common::Scratch {
    let bundle = common::temp_bundle("path-missing", label);
    let root = bundle.parent().unwrap().to_path_buf();
    if with_git {
        fs::create_dir_all(root.join(".git")).unwrap();
    }
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/lib.rs"), "x").unwrap();
    let mut dirs: Vec<&str> = records
        .iter()
        .filter_map(|(rel, _)| rel.split('/').next())
        .collect();
    dirs.sort_unstable();
    dirs.dedup();
    let links: String = dirs
        .iter()
        .map(|dir| format!("- [{dir}]({dir}/index.md)\n"))
        .collect();
    write(&bundle, "index.md", &format!("# Index\n\n{links}"));
    for (rel, contents) in records {
        write(&bundle, rel, contents);
    }
    common::index_bundle(&bundle);
    bundle
}

fn check_json(bundle: &Path, extra: &[&str]) -> (Option<i32>, Value) {
    let output = living_docs()
        .args(["check", "--json"])
        .args(extra)
        .arg("--")
        .arg(bundle)
        .output()
        .expect("failed to run living-docs check");
    let json = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    (output.status.code(), json)
}

fn findings(json: &Value) -> Vec<(String, String)> {
    json["advisories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "path-missing")
        .map(|a| {
            (
                a["file"].as_str().unwrap().to_owned(),
                a["message"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn advised(body: &str) -> Vec<(String, String)> {
    let bundle = repository("one", &[("adr/0001-x.md", adr(body))], true);
    findings(&check_json(&bundle, &[]).1)
}

#[test]
fn advises_a_missing_path_naming_the_path_and_the_record() {
    let found = advised("See `src/gone.rs`.");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].0.ends_with("adr/0001-x.md"), "{found:?}");
    assert!(found[0].1.contains("src/gone.rs"), "{found:?}");
}

#[test]
fn advises_a_path_with_a_line_suffix_once() {
    for token in ["src/gone.rs:42", "src/gone.rs:42:7", "./src/gone.rs"] {
        let found = advised(&format!("See `{token}`."));
        assert_eq!(found.len(), 1, "{token}: {found:?}");
        assert!(found[0].1.contains("src/gone.rs"), "{token}: {found:?}");
        assert!(!found[0].1.contains("./src"), "{token}: {found:?}");
    }
}

#[test]
fn advises_a_missing_directory() {
    assert_eq!(advised("See `src/missing_dir/`.").len(), 1);
}

#[test]
fn advises_a_path_named_twice_once() {
    assert_eq!(advised("`src/gone.rs` and again `src/gone.rs:3`.").len(), 1);
}

#[test]
fn advises_nothing_for_an_existing_path_and_clears_once_created() {
    assert!(advised("See `src/lib.rs`.").is_empty());
    let bundle = repository(
        "created",
        &[("adr/0001-x.md", adr("See `src/gone.rs`."))],
        true,
    );
    assert_eq!(findings(&check_json(&bundle, &[]).1).len(), 1);
    fs::write(bundle.parent().unwrap().join("src/gone.rs"), "x").unwrap();
    assert!(findings(&check_json(&bundle, &[]).1).is_empty());
}

#[test]
fn ignores_a_token_that_is_not_a_repository_path() {
    let tokens = [
        "adr/0001",
        "write/read",
        "true/false",
        "living-docs write new",
        "https://example.com/a/b",
        "~/.config/x",
        "a*b/c",
    ];
    for token in tokens {
        let found = advised(&format!("See `{token}`."));
        assert!(found.is_empty(), "{token}: {found:?}");
    }
}

#[test]
fn ignores_a_path_outside_inline_code() {
    let fenced = advised("```\nsrc/gone.rs\n```\n\n`ok`");
    assert!(fenced.is_empty(), "{fenced:?}");
    let link = advised("See [the file](src/gone.rs).");
    assert!(link.is_empty(), "{link:?}");
    let prose = advised("See src/gone.rs in prose.");
    assert!(prose.is_empty(), "{prose:?}");
}

#[test]
fn scope_examines_only_live_records() {
    let gone = "See `src/gone.rs`.";
    let records = [
        ("adr/0001-x.md", record("adr", "status: Superseded\n", gone)),
        ("adr/0002-y.md", record("adr", "status: Deprecated\n", gone)),
        ("issue/0001-z.md", record("issue", "status: closed\n", gone)),
        ("issue/0002-w.md", record("issue", "status: open\n", gone)),
    ];
    let bundle = repository("live", &records, true);
    let found = findings(&check_json(&bundle, &[]).1);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].0.ends_with("issue/0002-w.md"), "{found:?}");
}

#[test]
fn scope_follows_the_changed_files() {
    let records = [
        ("adr/0001-x.md", adr("See `src/gone.rs`.")),
        ("adr/0002-y.md", adr("Nothing.")),
    ];
    let bundle = repository("changed", &records, true);
    let json_for = |changed: &str| {
        check_json(
            &bundle,
            &["--changed-files", bundle.join(changed).to_str().unwrap()],
        )
        .1
    };
    assert_eq!(findings(&json_for("adr/0001-x.md")).len(), 1);
    assert!(findings(&json_for("adr/0002-y.md")).is_empty());
}

#[test]
fn gate_stays_open_and_the_kind_is_documented() {
    let bundle = repository(
        "gate",
        &[("adr/0001-x.md", adr("See `src/gone.rs`."))],
        true,
    );
    let (code, json) = check_json(&bundle, &[]);
    assert_eq!(code, Some(0), "{json}");
    assert_eq!(json["ok"], true);
    assert_eq!(findings(&json).len(), 1);
    let help = living_docs().args(["check", "--help"]).output().unwrap();
    let help = String::from_utf8_lossy(&help.stdout).to_string();
    assert!(help.contains("path-missing"), "{help}");
}

#[test]
fn gate_skips_the_rule_without_a_repository() {
    let bundle = repository(
        "norepo",
        &[("adr/0001-x.md", adr("See `src/gone.rs`."))],
        false,
    );
    assert!(
        !bundle.ancestors().any(|dir| dir.join(".git").exists()),
        "precondition: the temporary directory must have no .git ancestor"
    );
    let (code, json) = check_json(&bundle, &[]);
    assert_eq!(code, Some(0), "{json}");
    assert!(findings(&json).is_empty());
}

#[cfg(unix)]
#[test]
fn refuses_a_token_that_is_absolute_or_leaves_the_repository() {
    let tokens = [
        "/etc/passwd",
        "/src/gone.rs",
        "/login",
        "//x",
        "/",
        "src/../../etc/passwd",
        "src/../src/lib.rs",
        "../sibling/file.rs",
        "src/link/anything",
    ];
    let body = format!(
        "{}\n\nAnd `src/gone.rs` and `src/gone_dir/a.rs`.",
        tokens.map(|t| format!("`{t}`")).join(" ")
    );
    let bundle = repository("refuses", &[("adr/0001-x.md", adr(&body))], true);
    let outside = common::Scratch::new("living-docs-path-missing-outside");
    std::os::unix::fs::symlink(&*outside, bundle.parent().unwrap().join("src/link")).unwrap();
    let found = findings(&check_json(&bundle, &[]).1);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].1.contains("`src/gone.rs`"), "{found:?}");
    assert!(found[1].1.contains("`src/gone_dir/a.rs`"), "{found:?}");
}

#[test]
fn scope_never_examines_an_index_file() {
    let token = "See `src/gone.rs`.";
    let records = [
        ("adr/0001-x.md", adr(token)),
        ("issue/0001-d.md", record("issue", "status: done\n", token)),
    ];
    let bundle = repository("index", &records, true);
    for index in ["adr/index.md", "index.md"] {
        let live = format!("---\ntype: ADR\nstatus: Accepted\n---\n\n{token}\n");
        fs::write(bundle.join(index), live).unwrap();
    }
    let found = findings(&check_json(&bundle, &[]).1);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].0.ends_with("adr/0001-x.md"), "{found:?}");
}
