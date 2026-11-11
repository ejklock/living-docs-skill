//! Help obeys the output contract: clap's text on a TTY or with `--plain`,
//! one minified JSON document off a TTY or with `--json`.

mod common;

use common::living_docs;
use serde_json::Value;
use std::process::Output;

const VERBS: [&str; 4] = ["write", "read", "manage", "check"];

fn run(args: &[&str]) -> Output {
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn json_of(args: &[&str]) -> Value {
    let output = run(args);
    let text = stdout_of(&output);
    assert_eq!(output.status.code(), Some(0), "{args:?} got:\n{text}");
    assert_eq!(text.trim_end().lines().count(), 1, "{args:?} got:\n{text}");
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{args:?} is not JSON ({err}):\n{text}"))
}

fn names(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is not an array in {value}"))
        .iter()
        .map(|item| item["name"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn longs(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is not an array in {value}"))
        .iter()
        .filter_map(|option| option["long"].as_str().map(str::to_owned))
        .collect()
}

fn nested_command<'a>(doc: &'a Value, path: &[&str]) -> &'a Value {
    let mut node = doc;
    for name in path {
        node = node["commands"]
            .as_array()
            .or_else(|| node["subcommands"].as_array())
            .and_then(|list| list.iter().find(|command| command["name"] == *name))
            .unwrap_or_else(|| panic!("no {name} command"));
    }
    node
}

#[test]
fn piped_root_help_is_one_json_line_naming_every_verb_and_hiding_retired_names() {
    let doc = json_of(&["--help"]);

    assert_eq!(doc["name"], "living-docs");
    let commands = names(&doc, "commands");
    for verb in VERBS {
        assert!(
            commands.contains(&verb.to_owned()),
            "missing {verb}: {commands:?}"
        );
    }
    for alias in ["effective", "skill", "help"] {
        assert!(
            !commands.contains(&alias.to_owned()),
            "{alias} listed: {commands:?}"
        );
    }
}

#[test]
fn piped_root_help_lists_the_four_verbs_without_groups() {
    let doc = json_of(&["--help"]);

    assert!(doc.get("groups").is_none(), "groups present: {doc}");
    assert_eq!(names(&doc, "commands"), VERBS);
}

fn assert_contract(command: &Value, trail: &str) {
    let name = format!("{trail} {}", command["name"].as_str().unwrap_or_default());
    for field in ["about", "usage"] {
        assert!(
            command[field].as_str().is_some_and(|text| !text.is_empty()),
            "{name} lacks {field}"
        );
    }
    let has_examples = command["examples"]
        .as_array()
        .is_some_and(|list| !list.is_empty());
    assert!(has_examples, "{name} lacks examples");
    let codes: Vec<i64> = command["exit_codes"]
        .as_array()
        .unwrap_or_else(|| panic!("{name} lacks exit_codes"))
        .iter()
        .filter_map(|entry| entry["code"].as_i64())
        .collect();
    assert_eq!(codes, [0, 1, 2], "{name} exit codes");
    for sub in command["subcommands"].as_array().into_iter().flatten() {
        assert_contract(sub, &name);
    }
}

#[test]
fn every_operation_carries_its_full_contract() {
    let doc = json_of(&["--help"]);

    for command in doc["commands"].as_array().expect("commands array") {
        assert_contract(command, "");
    }
    for path in [
        ["manage", "install", "skills"],
        ["manage", "install", "hooks"],
        ["manage", "uninstall", "hooks"],
    ] {
        let leaf = nested_command(&doc, &path);
        assert!(
            leaf["examples"].as_array().is_some_and(|l| !l.is_empty()),
            "{path:?}"
        );
    }
    assert_eq!(doc["exit_codes"].as_array().map(Vec::len), Some(3));
}

#[test]
fn new_lists_its_positionals_and_options_with_the_documented_fields() {
    let doc = json_of(&["--help"]);
    let new = nested_command(&doc, &["write", "new"]);

    let type_arg = &new["args"][0];
    assert_eq!(type_arg["name"], "TYPE");
    assert_eq!(type_arg["required"], true);
    assert!(type_arg["possible_values"].is_array());
    let body = new["options"]
        .as_array()
        .and_then(|list| list.iter().find(|option| option["long"] == "body"))
        .expect("--body option");
    assert_eq!(body["value"], "PATH");
    assert_eq!(body["global"], false);
    assert!(body["default"].is_null());
    let first_example = new["examples"][0].as_str().unwrap_or_default();
    assert!(
        first_example.starts_with("living-docs write new adr"),
        "{first_example}"
    );
}

#[test]
fn install_and_uninstall_carry_their_nested_subcommands() {
    let doc = json_of(&["--help"]);

    assert_eq!(
        names(nested_command(&doc, &["manage", "install"]), "subcommands"),
        ["skills", "hooks"]
    );
    assert_eq!(
        names(
            nested_command(&doc, &["manage", "uninstall"]),
            "subcommands"
        ),
        ["hooks"]
    );
}

#[test]
fn root_help_lists_the_global_options() {
    let doc = json_of(&["--help"]);

    let listed = longs(&doc, "options");
    for long in ["docs-dir", "json", "plain", "color", "quiet"] {
        assert!(
            listed.contains(&long.to_owned()),
            "missing --{long}: {listed:?}"
        );
    }
    assert_eq!(doc["options"][0]["global"], true);
}

#[test]
fn verb_help_is_that_verbs_object_plus_the_global_options() {
    let doc = json_of(&["write", "set", "--help"]);

    assert_eq!(doc["name"], "set");
    assert_eq!(names(&doc, "args"), ["REFERENCE", "KEY", "VALUE"]);
    assert!(longs(&doc, "options").contains(&"from".to_owned()));
    let globals = longs(&doc, "global_options");
    assert!(globals.contains(&"docs-dir".to_owned()), "{globals:?}");
    assert!(globals.contains(&"json".to_owned()), "{globals:?}");
}

#[test]
fn nested_subcommand_help_names_its_own_options() {
    let doc = json_of(&["manage", "install", "skills", "--help"]);

    assert_eq!(doc["name"], "skills");
    let harness = doc["options"]
        .as_array()
        .and_then(|list| list.iter().find(|option| option["long"] == "harness"))
        .expect("--harness option");
    assert_eq!(harness["possible_values"][0], "claude");
    assert_eq!(harness["default"], "claude");
}

#[test]
fn short_flag_and_help_verb_also_print_json() {
    assert_eq!(json_of(&["write", "set", "-h"])["name"], "set");
    assert_eq!(json_of(&["help", "write", "set"])["name"], "set");
}

#[test]
fn json_flag_forces_the_document() {
    assert_eq!(json_of(&["--json", "check", "--help"])["name"], "check");
}

#[test]
fn plain_flag_keeps_clap_text() {
    let output = run(&["--help", "--plain"]);
    let text = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0));
    assert!(text.contains("Usage:"), "got:\n{text}");
    assert!(!text.trim_start().starts_with('{'), "got:\n{text}");
}

#[test]
fn color_always_keeps_clap_text() {
    let output = run(&["write", "set", "--help", "--color=always"]);

    assert!(stdout_of(&output).contains("Usage:"));
}

#[test]
fn piped_version_is_a_json_document_with_the_package_version() {
    let doc = json_of(&["--version"]);

    assert_eq!(doc["name"], "living-docs");
    assert_eq!(doc["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn plain_version_keeps_clap_text() {
    let output = run(&["--version", "--plain"]);

    assert_eq!(
        stdout_of(&output).trim(),
        format!("living-docs {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_verb_still_exits_two_with_text_on_stderr() {
    let output = run(&["no-such-verb"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(stdout_of(&output).is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no-such-verb"));
}
