//! `manage view <view> --export tla`: the TLA+ module of a flat state view on
//! standard output, and the refusals.

use std::path::Path;
use std::process::Output;

mod common;
use common::{living_docs, stdout_of, write};

const FLAT: &str = "stateDiagram-v2\n[*] --> pending\npending --> paid\npaid --> [*]\n";

fn view(kind: &str, fence: &str) -> String {
    format!(
        "---\ntype: Architecture View\ntitle: Order Lifecycle\ndescription: The lifecycle.\nkind: {kind}\ntimestamp: 2026-11-20T10:00:00Z\n---\n\n# Order Lifecycle\n\n```mermaid\n{fence}```\n\n## Drift instrument\n\nDrift caught by: the check.\n"
    )
}

fn bundle(label: &str, kind: &str, fence: &str) -> common::Scratch {
    let bundle = common::temp_bundle("view-export", label);
    write(
        &bundle,
        "architecture/order-lifecycle.md",
        &view(kind, fence),
    );
    bundle
}

fn export(bundle: &Path, reference: &str, flag_value: &str) -> Output {
    living_docs()
        .args(["--docs-dir"])
        .arg(bundle)
        .args(["manage", "view", reference, "--export", flag_value])
        .output()
        .expect("failed to run living-docs manage view")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn a_flat_state_view_prints_its_module_and_exits_zero() {
    let bundle = bundle("flat", "state", FLAT);
    let output = export(&bundle, "architecture/order-lifecycle", "tla");
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let module = stdout_of(&output);
    assert!(
        module.starts_with("---- MODULE order_lifecycle ----\n"),
        "{module}"
    );
    assert!(module.contains("Spec == Init /\\ [][Next]_state /\\ WF_state(Next)"));
}

#[test]
fn a_composite_state_is_refused_by_name() {
    let fence = "stateDiagram-v2\n[*] --> Outer\nstate Outer {\n[*] --> Inner\nInner --> [*]\n}\nOuter --> [*]\n";
    let bundle = bundle("composite", "state", fence);
    let output = export(&bundle, "architecture/order-lifecycle", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("composite state 'Outer'"),
        "{}",
        stderr_of(&output)
    );
    assert_eq!(stdout_of(&output), "");
}

#[test]
fn concurrent_regions_are_refused() {
    let fence = "stateDiagram-v2\n[*] --> A\nstate A {\nB --> C\n--\nD --> E\n}\n";
    let bundle = bundle("regions", "state", fence);
    let output = export(&bundle, "architecture/order-lifecycle", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("concurrent regions"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn a_view_that_is_not_a_state_view_is_refused() {
    let bundle = bundle("kind", "flow", FLAT);
    let output = export(&bundle, "architecture/order-lifecycle", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("not a state view"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn a_slug_without_a_letter_is_refused_naming_the_view() {
    let bundle = common::temp_bundle("view-export", "digits");
    write(&bundle, "architecture/2024.md", &view("state", FLAT));
    let output = export(&bundle, "architecture/2024", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("architecture/2024"),
        "{}",
        stderr_of(&output)
    );
    assert!(
        stderr_of(&output).contains("letter"),
        "{}",
        stderr_of(&output)
    );
    assert_eq!(stdout_of(&output), "");
}

#[test]
fn a_view_whose_diagram_does_not_parse_names_the_parse_failure() {
    let bundle = bundle("parse", "state", "stateDiagram-v2\n[*] --> order-paid\n");
    let output = export(&bundle, "architecture/order-lifecycle", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("does not parse"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn a_view_with_several_state_diagrams_is_refused() {
    let two = format!("{FLAT}```\n\n```mermaid\n{FLAT}");
    let bundle = bundle("several", "state", &two);
    let output = export(&bundle, "architecture/order-lifecycle", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("several state diagrams"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn an_unknown_view_is_refused() {
    let bundle = bundle("unknown", "state", FLAT);
    let output = export(&bundle, "architecture/missing", "tla");
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr_of(&output).contains("no record found"),
        "{}",
        stderr_of(&output)
    );
}

#[test]
fn an_export_format_other_than_tla_is_refused_by_the_parser() {
    let bundle = bundle("format", "state", FLAT);
    let output = export(&bundle, "architecture/order-lifecycle", "alloy");
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr_of(&output).contains("alloy"),
        "{}",
        stderr_of(&output)
    );
}
