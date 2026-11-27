//! `manage index` and `write new` add the `okf_version` marker to a bundle
//! root that lacks it, leaving the rest of the file as written.

mod common;

use common::{living_docs, run_check, run_new, temp_bundle, write, Scratch};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const ALPHA: &str =
    "---\ntype: ADR\ntitle: Alpha\nstatus: accepted\n---\n\n# 0001 — Alpha\n\nBody.\n";
const HAND_ROOT: &str = "# Project docs\n";
const MARKED_ROOT: &str = "---\nokf_version: \"0.1\"\n---\n\n# Project docs\n";

fn unmarked_bundle(label: &str) -> Scratch {
    let docs = temp_bundle("root-marker", label);
    assert!(run_new(&docs, "adr", "Alpha").status.success());
    write(&docs, "index.md", HAND_ROOT);
    docs
}

fn index(docs: &Path) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "manage", "index"])
        .arg("--plain")
        .output()
        .unwrap()
}

fn root_text(docs: &Path) -> String {
    fs::read_to_string(docs.join("index.md")).unwrap()
}

fn root_is_listed(docs: &Path, output: &Output) -> bool {
    let root = docs.join("index.md");
    String::from_utf8_lossy(&output.stdout).contains(root.to_str().unwrap())
}

fn hook_output(project: &Path) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/session-context.sh");
    let output = Command::new("bash")
        .arg(script)
        .env("LIVING_DOCS_PLUGIN", "1")
        .env_remove("LIVING_DOCS_BUNDLE")
        .env("CLAUDE_PROJECT_DIR", project)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn manage_index_adds_the_marker_to_a_hand_written_root_once() {
    let docs = unmarked_bundle("index");

    let first = index(&docs);
    assert!(first.status.success(), "{first:?}");
    assert!(
        root_text(&docs).starts_with(MARKED_ROOT),
        "{}",
        root_text(&docs)
    );
    assert!(root_is_listed(&docs, &first), "{first:?}");

    let second = index(&docs);
    assert!(second.status.success(), "{second:?}");
    assert!(!root_is_listed(&docs, &second), "{second:?}");
    let check = run_check(&docs);
    let report = String::from_utf8_lossy(&check.stdout);
    assert!(!report.contains("index.md"), "{report}");
}

#[test]
fn write_new_adds_the_marker_to_a_hand_written_root() {
    let docs = unmarked_bundle("new");

    let output = run_new(&docs, "adr", "x");

    assert!(output.status.success(), "{output:?}");
    assert!(
        root_text(&docs).starts_with(MARKED_ROOT),
        "{}",
        root_text(&docs)
    );
}

#[test]
fn session_hook_prints_only_after_the_marker_is_added() {
    let project = Scratch::new("root-marker-hook");
    let docs = project.join("docs");
    write(&docs, "adr/0001-alpha.md", ALPHA);
    write(&docs, "index.md", HAND_ROOT);
    assert_eq!(hook_output(&project), "");

    assert!(index(&docs).status.success());

    assert!(!hook_output(&project).is_empty());
}
