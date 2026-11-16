use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

mod common;
use common::Scratch;

const SCRIPT: &str = ".living-docs/hooks/decision-prompt.sh";
const SETTINGS: &str = ".claude/settings.json";

fn project(label: &str) -> Scratch {
    let project = Scratch::new(&format!("living-docs-hooks-stop-{label}"));
    fs::create_dir_all(project.join("docs")).unwrap();
    project
}

fn manage(verb: &str, dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["manage", verb, "hooks", "--dir", dir.to_str().unwrap()])
        .args(extra)
        .output()
        .expect("failed to run living-docs manage")
}

fn settings(dir: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(dir.join(SETTINGS)).expect("settings.json exists");
    serde_json::from_str(&raw).expect("settings.json parses")
}

fn section_commands(settings: &serde_json::Value, section: &str) -> Vec<String> {
    settings["hooks"][section]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|entry| entry["hooks"].as_array().into_iter().flatten())
        .filter_map(|hook| hook["command"].as_str().map(str::to_owned))
        .collect()
}

fn earlier_stop_entry() -> serde_json::Value {
    serde_json::json!({
        "hooks": { "Stop": [ { "hooks": [ { "type": "command", "command": format!("LIVING_DOCS_BUNDLE=docs \"$CLAUDE_PROJECT_DIR\"/{SCRIPT}") } ] } ] }
    })
}

fn own_stop_entry() -> serde_json::Value {
    serde_json::json!({
        "hooks": { "Stop": [ { "hooks": [ { "type": "command", "command": "echo mine" } ] } ] }
    })
}

fn earlier_plus_own_stop_entries() -> serde_json::Value {
    let mut value = earlier_stop_entry();
    value["hooks"]["Stop"].as_array_mut().unwrap().extend(
        own_stop_entry()["hooks"]["Stop"]
            .as_array()
            .unwrap()
            .clone(),
    );
    value
}

#[test]
fn install_writes_the_decision_prompt_script_executable_and_wires_one_post_tool_use_entry_for_edits_and_writes(
) {
    let project = project("wire");
    assert!(manage("install", &project, &[]).status.success());

    let mode = fs::metadata(project.join(SCRIPT))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
    let written = settings(&project);
    let entries = written["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "got: {entries:?}");
    assert_eq!(entries[0]["matcher"], "Edit|Write");
    assert!(written["hooks"].get("Stop").is_none(), "got: {written}");
    let commands = section_commands(&written, "PostToolUse");
    assert!(
        commands[0].starts_with("LIVING_DOCS_BUNDLE=docs "),
        "got: {commands:?}"
    );
    assert!(commands[0].ends_with(SCRIPT), "got: {commands:?}");
    let raw = fs::read_to_string(project.join(SETTINGS)).unwrap();
    assert!(!raw.contains("LIVING_DOCS_DECISION_PROMPT"));
}

#[test]
fn install_twice_leaves_one_post_tool_use_entry_and_keeps_a_users_own_stop_entry() {
    let project = project("idempotent");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(project.join(SETTINGS), own_stop_entry().to_string()).unwrap();

    assert!(manage("install", &project, &[]).status.success());
    assert!(manage("install", &project, &[]).status.success());

    let written = settings(&project);
    assert_eq!(section_commands(&written, "PostToolUse").len(), 1);
    assert_eq!(section_commands(&written, "Stop"), vec!["echo mine"]);
}

#[test]
fn install_removes_an_earlier_living_docs_stop_entry_and_keeps_a_users_own() {
    let project = project("earlier");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(SETTINGS),
        earlier_plus_own_stop_entries().to_string(),
    )
    .unwrap();

    assert!(manage("install", &project, &[]).status.success());

    let written = settings(&project);
    assert_eq!(section_commands(&written, "Stop"), vec!["echo mine"]);
    assert_eq!(section_commands(&written, "PostToolUse").len(), 1);
}

#[test]
fn uninstall_removes_an_earlier_living_docs_stop_entry_and_keeps_a_users_own() {
    let project = project("earlier-uninstall");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(SETTINGS),
        earlier_plus_own_stop_entries().to_string(),
    )
    .unwrap();

    assert!(manage("uninstall", &project, &[]).status.success());

    assert_eq!(settings(&project), own_stop_entry());
}

#[test]
fn dry_run_writes_nothing_and_names_the_post_tool_use_wiring() {
    let project = project("dry");
    let output = manage("install", &project, &["--dry-run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("decision-prompt.sh"), "got: {stdout}");
    assert!(stdout.contains("PostToolUse"), "got: {stdout}");
    assert!(!project.join(SCRIPT).exists());
    assert!(!project.join(SETTINGS).exists());
}

#[test]
fn uninstall_restores_settings_and_removes_the_script() {
    let project = project("uninstall");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(project.join(SETTINGS), own_stop_entry().to_string()).unwrap();
    let before = settings(&project);

    assert!(manage("install", &project, &[]).status.success());
    assert!(manage("uninstall", &project, &[]).status.success());

    assert!(!project.join(SCRIPT).exists());
    assert_eq!(settings(&project), before);
}

#[test]
fn uninstall_leaves_settings_with_no_hooks_object_when_install_created_it() {
    let project = project("created");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(project.join(SETTINGS), r#"{"x":1}"#).unwrap();

    assert!(manage("install", &project, &[]).status.success());
    assert!(manage("uninstall", &project, &[]).status.success());

    assert_eq!(settings(&project), serde_json::json!({"x": 1}));
}

#[test]
fn uninstall_treats_a_users_empty_section_as_absent() {
    let project = project("empty-section");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(
        project.join(SETTINGS),
        r#"{"hooks":{"PostToolUse":[]},"x":1}"#,
    )
    .unwrap();

    assert!(manage("install", &project, &[]).status.success());
    assert!(manage("uninstall", &project, &[]).status.success());

    assert_eq!(settings(&project), serde_json::json!({"x": 1}));
}

#[test]
fn install_hooks_help_names_the_decision_prompt_nudge() {
    let output = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["manage", "install", "hooks", "--help"])
        .output()
        .expect("failed to run living-docs manage install hooks --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("decision-prompt"), "got: {stdout}");
}

#[test]
fn install_hooks_long_help_names_the_post_tool_use_wiring() {
    let output = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["manage", "install", "hooks", "--help", "--plain"])
        .output()
        .expect("failed to run living-docs manage install hooks --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("PostToolUse"), "got: {stdout}");
    assert!(!stdout.contains("Stop"), "got: {stdout}");
}
