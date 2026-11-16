use serde_json::Value;
use std::path::Path;

mod common;
use common::{living_docs, run_check, write};

fn temp_bundle(label: &str) -> common::Scratch {
    common::temp_bundle("link-style", label)
}

fn write_bundle(bundle: &Path, issue_body: &str, root_body: &str) {
    write(
        bundle,
        "index.md",
        &format!("# Index\n\n- [Issues](issues/index.md)\n- [ADRs](adr/index.md)\n\n{root_body}\n"),
    );
    write(bundle, "issues/index.md", "# Issues\n\n- [Y](0001-y.md)\n");
    write(bundle, "adr/index.md", "# ADRs\n\n- [A](0001-a.md)\n");
    write(
        bundle,
        "adr/0001-a.md",
        "---\ntype: Reference\ntitle: A\ndescription: Target.\n---\n# A\n\n## Context\n",
    );
    write(bundle, "assets/i.png", "png");
    write(
        bundle,
        "issues/0001-y.md",
        &format!(
            "---\ntype: Reference\ntitle: Y\ndescription: Linker.\n---\n# Y\n\n{issue_body}\n"
        ),
    );
    common::index_bundle(bundle);
}

fn check_json(bundle: &Path) -> (Option<i32>, Value) {
    let output = living_docs()
        .args(["check", bundle.to_str().unwrap(), "--json"])
        .output()
        .expect("failed to run living-docs check");
    let json = serde_json::from_slice(&output.stdout).expect("check --json prints JSON");
    (output.status.code(), json)
}

fn link_style_messages(json: &Value, file_suffix: &str) -> Vec<String> {
    json["advisories"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["kind"] == "link-style" && a["file"].as_str().unwrap().ends_with(file_suffix))
        .map(|a| a["message"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_rooted_link_or_image_is_advised_with_its_relative_form_and_nothing_else_is() {
    let bundle = temp_bundle("rooted");
    let body = "[a](/adr/0001-a.md) [b](../adr/0001-a.md) [c](https://example.com/x) \
[e](/adr/0001-a.md#context) ![i](/assets/i.png)\n\n\
```\n[f](/adr/0001-a.md)\n```";
    write_bundle(&bundle, body, "");

    let (code, json) = check_json(&bundle);
    let messages = link_style_messages(&json, "issues/0001-y.md");

    assert_eq!(code, Some(0), "an advisory never fails check: {json}");
    assert_eq!(messages.len(), 3, "got: {messages:?}");
    assert!(messages[0].contains("/adr/0001-a.md") && messages[0].contains("`../adr/0001-a.md`"));
    assert!(messages[1].contains("/adr/0001-a.md#context"));
    assert!(messages[1].contains("`../adr/0001-a.md#context`"));
    assert!(messages[2].contains("/assets/i.png") && messages[2].contains("`../assets/i.png`"));
}

#[test]
fn a_protocol_relative_link_is_not_advised_as_rooted() {
    let bundle = temp_bundle("protocol-relative");
    write_bundle(&bundle, "[d](//cdn.example.com/x)", "");

    let (_, json) = check_json(&bundle);

    assert!(link_style_messages(&json, "issues/0001-y.md").is_empty());
}

#[test]
fn a_rooted_link_from_the_bundle_root_names_the_form_without_parent_steps() {
    let bundle = temp_bundle("root-index");
    write_bundle(&bundle, "", "[a](/adr/0001-a.md)");

    let (code, json) = check_json(&bundle);
    let messages = link_style_messages(&json, "/index.md");

    assert_eq!(code, Some(0), "got: {json}");
    assert_eq!(messages.len(), 1, "got: {messages:?}");
    assert!(messages[0].contains("`adr/0001-a.md`"), "got: {messages:?}");
    assert!(!messages[0].contains("`../"), "got: {messages:?}");
}

#[test]
fn a_rooted_link_to_a_missing_target_is_advised_and_stays_a_violation() {
    let bundle = temp_bundle("missing");
    write_bundle(&bundle, "[gone](/adr/0099-gone.md)", "");

    let (code, json) = check_json(&bundle);
    let messages = link_style_messages(&json, "issues/0001-y.md");

    assert_eq!(code, Some(1), "the broken link still fails: {json}");
    assert_eq!(messages.len(), 1, "got: {messages:?}");
    assert!(messages[0].contains("`../adr/0099-gone.md`"));
}

#[test]
fn the_text_summary_lists_the_link_style_kind() {
    let bundle = temp_bundle("summary");
    write_bundle(&bundle, "[a](/adr/0001-a.md)", "");

    let output = run_check(&bundle);
    let stdout = common::stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "got:\n{stdout}");
    assert!(
        stdout
            .lines()
            .any(|l| l.trim() == "1 LINK-STYLE advisory: 0001"),
        "got:\n{stdout}"
    );
}

#[test]
fn a_rooted_link_to_the_linking_directory_is_advised_as_dot_slash() {
    let bundle = temp_bundle("own-directory");
    write_bundle(&bundle, "", "");
    write(&bundle, "adr/index.md", "# ADRs\n\n- [self](/adr/)\n");

    let (_, json) = check_json(&bundle);
    let messages = link_style_messages(&json, "adr/index.md");

    assert_eq!(messages.len(), 1, "got: {messages:?}");
    assert!(
        messages[0].contains("write `./` instead"),
        "got: {messages:?}"
    );
}
