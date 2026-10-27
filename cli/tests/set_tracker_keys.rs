mod common;

use common::{living_docs, run_check, run_new, stdout_of, temp_bundle};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

fn set(docs: &Path, args: &[&str]) -> Output {
    living_docs()
        .args(["--docs-dir", docs.to_str().unwrap(), "set"])
        .args(args)
        .arg("--plain")
        .output()
        .unwrap()
}

fn issue_bundle(label: &str) -> (common::Scratch, PathBuf) {
    let docs = temp_bundle("set-tracker-keys", label);
    assert!(run_new(&docs, "issue", "T").status.success());
    let path = docs.join("issues/0001-t.md");
    (docs, path)
}

fn lines_of(path: &Path, key: &str) -> Vec<String> {
    let prefix = format!("{key}:");
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.starts_with(&prefix))
        .map(str::to_string)
        .collect()
}

/// A freshly scaffolded issue carries its own unfilled-template findings, so
/// "check ok" means the findings are exactly those of the untouched scaffold.
fn scaffold_findings(docs: &Path) -> String {
    let owner = docs.parent().and_then(Path::file_name).unwrap();
    let label = format!("baseline-{}", owner.to_string_lossy());
    let scratch = temp_bundle("set-tracker-keys", &label);
    assert!(run_new(&scratch, "issue", "T").status.success());
    let text = stdout_of(&run_check(&scratch));
    text.replace(scratch.to_str().unwrap(), docs.to_str().unwrap())
}

fn assert_clean(docs: &Path) {
    assert_eq!(stdout_of(&run_check(docs)), scaffold_findings(docs));
    let fmt = living_docs()
        .args(["fmt", docs.to_str().unwrap(), "--check"])
        .output()
        .unwrap();
    assert!(fmt.status.success(), "fmt --check failed: {fmt:?}");
}

fn assert_refused_unchanged(docs: &Path, path: &Path, args: &[&str]) -> String {
    let before = fs::read_to_string(path).unwrap();
    let out = set(docs, args);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(fs::read_to_string(path).unwrap(), before);
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn tracker_writes_one_scalar_and_stays_canonical() {
    let (docs, path) = issue_bundle("tracker-number");
    assert!(set(&docs, &["0001", "tracker", "12"]).status.success());
    assert_eq!(lines_of(&path, "tracker"), ["tracker: 12"]);
    assert_clean(&docs);
}

#[test]
fn tracker_url_reads_back_as_the_exact_string() {
    let (docs, path) = issue_bundle("tracker-url");
    let url = "https://github.com/o/r/issues/12";
    assert!(set(&docs, &["0001", "tracker", url]).status.success());
    assert_eq!(lines_of(&path, "tracker"), [format!("tracker: {url}")]);
    assert_clean(&docs);
}

#[test]
fn tracker_second_set_replaces_never_appends() {
    let (docs, path) = issue_bundle("tracker-twice");
    assert!(set(&docs, &["0001", "tracker", "12"]).status.success());
    assert!(set(&docs, &["0001", "tracker", "13"]).status.success());
    assert_eq!(lines_of(&path, "tracker"), ["tracker: 13"]);
}

#[test]
fn tracker_empty_value_is_refused_and_file_unchanged() {
    let (docs, path) = issue_bundle("tracker-empty");
    let stderr = assert_refused_unchanged(&docs, &path, &["0001", "tracker", ""]);
    assert!(stderr.contains("tracker"), "got: {stderr}");
}

#[test]
fn tracker_value_with_a_newline_is_refused_and_file_unchanged() {
    let (docs, path) = issue_bundle("tracker-newline");
    fs::write(docs.join("v.txt"), "12\n13\n").unwrap();
    let file = docs.join("v.txt");
    let args = ["0001", "tracker", "--from", file.to_str().unwrap()];
    let stderr = assert_refused_unchanged(&docs, &path, &args);
    assert!(stderr.contains("tracker"), "got: {stderr}");
}

#[test]
fn list_labels_write_a_flow_list_trimmed_and_in_order() {
    let (docs, path) = issue_bundle("list-labels");
    let out = set(&docs, &["0001", "labels", "slice, skeleton"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(lines_of(&path, "labels"), ["labels: [slice, skeleton]"]);
    assert_clean(&docs);
}

#[test]
fn list_blocked_by_keeps_entries_as_written_in_order() {
    let (docs, path) = issue_bundle("list-blocked");
    let out = set(&docs, &["0001", "blocked_by", "0002,0003"]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: [0002, 0003]"]);
    assert_clean(&docs);
}

#[test]
fn list_one_item_is_a_one_entry_list() {
    let (docs, path) = issue_bundle("list-one");
    assert!(set(&docs, &["0001", "labels", "solo"]).status.success());
    assert_eq!(lines_of(&path, "labels"), ["labels: [solo]"]);
    assert_clean(&docs);
}

#[test]
fn list_empty_value_is_an_empty_list_and_replaces() {
    let (docs, path) = issue_bundle("list-empty");
    assert!(set(&docs, &["0001", "blocked_by", "0002"]).status.success());
    assert!(set(&docs, &["0001", "blocked_by", ""]).status.success());
    assert_eq!(lines_of(&path, "blocked_by"), ["blocked_by: []"]);
    assert_clean(&docs);
}

#[test]
fn list_drops_empty_items_and_trims() {
    let (docs, path) = issue_bundle("list-drop");
    let out = set(&docs, &["0001", "labels", "  a,,b ,  "]);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(lines_of(&path, "labels"), ["labels: [a, b]"]);
    assert_clean(&docs);
}

#[test]
fn refuse_list_items_the_flow_list_cannot_carry() {
    let (docs, path) = issue_bundle("refuse-items");
    for (key, value) in [("labels", "a{b"), ("labels", "a}b"), ("blocked_by", "x,y}")] {
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", key, value]);
        assert!(stderr.contains(key), "{value}: {stderr}");
    }
}

#[test]
fn comment_opener_values_are_quoted_and_read_back_exactly() {
    let (docs, path) = issue_bundle("comment-opener");
    assert!(set(&docs, &["0001", "tracker", "12 #x"]).status.success());
    assert_eq!(lines_of(&path, "tracker"), ["tracker: \"12 #x\""]);
    assert!(set(&docs, &["0001", "labels", "a #b, c"]).status.success());
    assert_eq!(lines_of(&path, "labels"), ["labels: [\"a #b\", c]"]);
    assert_clean(&docs);
}

#[test]
fn refuse_leaves_values_that_read_back_exactly_accepted() {
    let (docs, path) = issue_bundle("refuse-accepts");
    for (key, value, line) in [
        ("labels", "a#b", "labels: [a#b]"),
        ("labels", "a:b, x: y", "labels: [a:b, \"x: y\"]"),
        ("tracker", "https://h/x#c-1", "tracker: https://h/x#c-1"),
        ("tracker", "#12", "tracker: \"#12\""),
    ] {
        let out = set(&docs, &["0001", key, value]);
        assert!(out.status.success(), "{value}: {out:?}");
        assert_eq!(lines_of(&path, key), [line]);
        assert_clean(&docs);
    }
}

#[test]
fn refuse_a_block_style_list_and_replace_a_flow_list() {
    let (docs, path) = issue_bundle("refuse-block");
    assert!(set(&docs, &["0001", "labels", "a, b"]).status.success());
    let flow = fs::read_to_string(&path).unwrap();
    let block = flow.replace("labels: [a, b]", "labels:\n  - a\n  - b");
    fs::write(&path, &block).unwrap();
    let stderr = assert_refused_unchanged(&docs, &path, &["0001", "labels", "c"]);
    assert!(stderr.contains("labels"), "got: {stderr}");

    fs::write(&path, &flow).unwrap();
    assert!(set(&docs, &["0001", "labels", "c"]).status.success());
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        flow.replace("labels: [a, b]", "labels: [c]")
    );
}

#[test]
fn list_item_with_brackets_or_quotes_is_refused_naming_the_key() {
    let (docs, path) = issue_bundle("list-unsupported");
    for value in ["a[b", "a]b", "\"a,b\""] {
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", "labels", value]);
        assert!(stderr.contains("labels"), "got: {stderr}");
    }
}

#[test]
fn list_write_changes_only_its_own_key() {
    let (docs, path) = issue_bundle("list-neighbors");
    assert!(set(&docs, &["0001", "labels", "a"]).status.success());
    let with_labels = fs::read_to_string(&path).unwrap();
    assert!(set(&docs, &["0001", "tracker", "7"]).status.success());
    let after = fs::read_to_string(&path).unwrap();
    let others: Vec<&str> = after
        .lines()
        .filter(|line| !line.starts_with("tracker:"))
        .collect();
    assert_eq!(others, with_labels.lines().collect::<Vec<_>>());
}

#[test]
fn gate_refuses_each_key_on_an_adr_naming_key_and_issue_type() {
    let docs = temp_bundle("set-tracker-keys", "gate-adr");
    assert!(run_new(&docs, "adr", "A").status.success());
    let path = docs.join("adr/0001-a.md");
    for key in ["tracker", "labels", "blocked_by"] {
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", key, "x"]);
        assert!(stderr.contains(key), "got: {stderr}");
        assert!(stderr.contains("Issue"), "got: {stderr}");
    }
}

#[test]
fn refuse_values_the_parser_would_store_as_something_else() {
    let (docs, path) = issue_bundle("refuse-reinterpreted");
    for (key, value) in [
        ("labels", "1e3"),
        ("labels", "null, a"),
        ("blocked_by", "~"),
    ] {
        let stderr = assert_refused_unchanged(&docs, &path, &["0001", key, value]);
        assert!(stderr.contains(key), "{value}: {stderr}");
    }
}

#[test]
fn a_tracker_that_reads_as_a_number_when_plain_is_written_quoted() {
    let (docs, path) = issue_bundle("tracker-quoted-number");
    assert!(set(&docs, &["0001", "tracker", "0x10"]).status.success());
    assert_eq!(lines_of(&path, "tracker"), ["tracker: \"0x10\""]);
    assert_clean(&docs);
}

#[test]
fn help_names_the_issue_keys_as_settable() {
    let out = living_docs()
        .args(["set", "--help", "--json"])
        .output()
        .unwrap();
    let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let key_help = doc["args"]
        .as_array()
        .unwrap()
        .iter()
        .find(|arg| arg["name"] == "KEY")
        .and_then(|arg| arg["help"].as_str())
        .unwrap();
    assert!(
        key_help.starts_with(
            "one of: status, description, owner, tracker, labels, blocked_by, title, number, section."
        ),
        "got: {key_help}"
    );
}

#[test]
fn gate_existing_keys_behave_as_before_and_unknown_key_lists_the_new_ones() {
    let (docs, path) = issue_bundle("gate-existing");
    assert!(set(&docs, &["0001", "owner", "a@b.c"]).status.success());
    assert_eq!(lines_of(&path, "owner"), ["owner: a@b.c"]);
    let out = set(&docs, &["0001", "bogus", "x"]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tracker, labels, blocked_by"),
        "got: {stderr}"
    );
}
