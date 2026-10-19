//! The JSON help describes a boolean flag as a flag (no value, no default)
//! and a value option with its value name; the two-token `--color always`
//! form keeps clap's text like `--color=always`.

mod common;

use common::living_docs;
use serde_json::Value;
use std::process::Output;

fn run(args: &[&str]) -> Output {
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs")
}

fn json_of(args: &[&str]) -> Value {
    let output = run(args);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_eq!(output.status.code(), Some(0), "{args:?} got:\n{text}");
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("{args:?} is not JSON ({err}):\n{text}"))
}

fn command_named<'a>(doc: &'a Value, name: &str) -> &'a Value {
    doc["commands"]
        .as_array()
        .and_then(|list| list.iter().find(|command| command["name"] == name))
        .unwrap_or_else(|| panic!("no {name} command"))
}

fn option_named<'a>(options: &'a Value, long: &str) -> &'a Value {
    options
        .as_array()
        .and_then(|list| list.iter().find(|option| option["long"] == long))
        .unwrap_or_else(|| panic!("no {long} option in {options}"))
}

fn collect_options<'a>(node: &'a Value, found: &mut Vec<&'a Value>) {
    if let Some(options) = node["options"].as_array() {
        found.extend(options);
    }
    if let Some(children) = node["commands"].as_array() {
        for child in children {
            collect_options(child, found);
        }
    }
}

#[test]
fn a_flag_renders_without_value_or_default() {
    let doc = json_of(&["--help"]);
    let check = option_named(&command_named(&doc, "fmt")["options"], "check");

    assert!(check["value"].is_null(), "got {check}");
    assert!(check["default"].is_null(), "got {check}");
}

#[test]
fn a_value_option_keeps_its_value_name() {
    let doc = json_of(&["--help"]);
    let body = option_named(&command_named(&doc, "new")["options"], "body");

    assert_eq!(body["value"], "PATH");
}

#[test]
fn a_global_flag_renders_without_value() {
    let doc = json_of(&["--help"]);

    assert!(option_named(&doc["options"], "quiet")["value"].is_null());
}

#[test]
fn no_option_without_a_value_carries_a_default() {
    let doc = json_of(&["--help"]);
    let mut found = Vec::new();
    collect_options(&doc, &mut found);

    assert!(!found.is_empty());
    assert!(
        found.iter().any(|o| o["value"].is_null()),
        "no flag in the document"
    );
    for option in found.iter().filter(|option| option["value"].is_null()) {
        assert!(option["default"].is_null(), "got {option}");
    }
}

#[test]
fn two_token_color_always_keeps_clap_text() {
    let output = run(&["--color", "always", "set", "--help"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
}
