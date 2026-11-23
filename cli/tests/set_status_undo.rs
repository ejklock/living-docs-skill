mod common;

use common::{
    living_docs, new_two_adrs_and_supersede, run_check, run_new, run_supersede, stdout_of,
    temp_bundle,
};
use std::fs;
use std::path::Path;
use std::process::Output;

fn run(docs: &Path, verb: &str, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), verb])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn in_force_numbers(docs: &Path) -> Vec<String> {
    let dir = docs.to_str().unwrap();
    let args = ["--docs-dir", dir, "read", "records", "--json"];
    let out = living_docs().args(args).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let records = json["records"].as_array().unwrap();
    records.iter().map(|r| r["number"].to_string()).collect()
}

fn fill_template_slots(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    let filled: Vec<&str> = text.lines().filter(|l| !l.contains("{{")).collect();
    let filled = filled.join("\n").replace(
        "description: <One sentence — the decision and its scope.>",
        "description: A decision.",
    );
    fs::write(path, format!("{filled}\n")).unwrap();
}

fn edit(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "no {from:?} in {text}");
    fs::write(path, text.replace(from, to)).unwrap();
}

fn repair_with_the_cli(docs: &Path) {
    assert!(run(docs, "manage", &["fmt", docs.to_str().unwrap()])
        .status
        .success());
    assert!(run(docs, "manage", &["index"]).status.success());
}

#[test]
fn undo_refusal_names_both_edits_and_both_repair_commands() {
    let docs = temp_bundle("set-status-undo", "message");
    new_two_adrs_and_supersede(&docs);
    let out = run(&docs, "write", &["set", "adr/0001", "status", "Accepted"]);
    let message = String::from_utf8_lossy(&out.stderr).to_string();
    for needle in ["superseded_by", "supersedes", "manage fmt", "manage index"] {
        assert!(message.contains(needle), "missing {needle}: {message}");
    }
    let clause = message.split_once("supersedes").unwrap().1;
    assert!(clause.contains("0002"), "{message}");
    assert!(message.trim().lines().count() == 1, "{message}");
    assert!(!message.contains("TYPE"), "{message}");
    let text = message.split_once(".md: ").unwrap().1;
    assert!(text.len() < 400, "{} chars: {text}", text.len());
}

#[test]
fn undo_by_hand_as_the_refusal_says_leaves_a_clean_bundle() {
    let docs = temp_bundle("set-status-undo", "by-hand");
    new_two_adrs_and_supersede(&docs);
    let old = docs.join("adr/0001-old-decision.md");
    let new = docs.join("adr/0002-new-decision.md");
    fill_template_slots(&old);
    fill_template_slots(&new);
    edit(&old, "superseded_by: 0002\n", "");
    edit(&old, "status: Superseded", "status: Accepted");
    edit(&new, "supersedes: 0001\n", "");
    repair_with_the_cli(&docs);
    let check = run_check(&docs);
    assert_eq!(check.status.code(), Some(0), "{}", stdout_of(&check));
    let text = fs::read_to_string(&old).unwrap();
    assert!(!text.contains("SUPERSEDED"), "callout left: {text}");
    assert_eq!(in_force_numbers(&docs), ["1", "2"]);
}

fn line(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).unwrap();
    text.lines().find(|l| l.starts_with(key)).map(str::to_owned)
}

#[test]
fn undo_keeps_the_other_number_when_the_successor_supersedes_two() {
    let docs = temp_bundle("set-status-undo", "two");
    new_two_adrs_and_supersede(&docs);
    assert!(run_new(&docs, "adr", "Third").status.success());
    succeeded(&run_supersede(&docs, "adr/0003", "adr/0002"));
    let old = docs.join("adr/0001-old-decision.md");
    let new = docs.join("adr/0002-new-decision.md");
    let third = docs.join("adr/0003-third.md");
    for path in [&old, &new, &third] {
        fill_template_slots(path);
    }
    edit(&new, "supersedes: [0001, 0003]", "supersedes: [0003]");
    edit(&old, "superseded_by: 0002\n", "");
    edit(&old, "status: Superseded", "status: Accepted");
    repair_with_the_cli(&docs);
    let check = run_check(&docs);
    assert_eq!(check.status.code(), Some(0), "{}", stdout_of(&check));
    assert_eq!(
        line(&new, "supersedes:").as_deref(),
        Some("supersedes: 0003")
    );
    assert_eq!(
        line(&third, "superseded_by:").as_deref(),
        Some("superseded_by: 0002")
    );
    assert_eq!(
        line(&third, "status:").as_deref(),
        Some("status: Superseded")
    );
    assert_eq!(in_force_numbers(&docs), ["1", "2"]);
}

fn succeeded(out: &Output) {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
