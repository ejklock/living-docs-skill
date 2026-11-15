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

fn stop_commands(settings: &serde_json::Value) -> Vec<String> {
    settings["hooks"]["Stop"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|entry| entry["hooks"].as_array().into_iter().flatten())
        .filter_map(|hook| hook["command"].as_str().map(str::to_owned))
        .collect()
}

fn own_stop_entry() -> serde_json::Value {
    serde_json::json!({
        "hooks": { "Stop": [ { "hooks": [ { "type": "command", "command": "echo mine" } ] } ] }
    })
}

#[test]
fn install_writes_the_decision_prompt_script_executable_and_wires_one_stop_entry() {
    let project = project("wire");
    assert!(manage("install", &project, &[]).status.success());

    let mode = fs::metadata(project.join(SCRIPT))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
    let commands = stop_commands(&settings(&project));
    assert_eq!(commands.len(), 1, "got: {commands:?}");
    assert!(
        commands[0].starts_with("LIVING_DOCS_BUNDLE=docs "),
        "got: {commands:?}"
    );
    assert!(commands[0].ends_with(SCRIPT), "got: {commands:?}");
    let raw = fs::read_to_string(project.join(SETTINGS)).unwrap();
    assert!(!raw.contains("LIVING_DOCS_DECISION_PROMPT"));
}

#[test]
fn install_twice_leaves_one_stop_entry_and_keeps_a_users_own() {
    let project = project("idempotent");
    fs::create_dir_all(project.join(".claude")).unwrap();
    fs::write(project.join(SETTINGS), own_stop_entry().to_string()).unwrap();

    assert!(manage("install", &project, &[]).status.success());
    assert!(manage("install", &project, &[]).status.success());

    let commands = stop_commands(&settings(&project));
    assert_eq!(commands.len(), 2, "got: {commands:?}");
    assert!(commands.contains(&"echo mine".to_owned()));
}

#[test]
fn dry_run_writes_nothing_and_names_the_stop_wiring() {
    let project = project("dry");
    let output = manage("install", &project, &["--dry-run"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(stdout.contains("decision-prompt.sh"), "got: {stdout}");
    assert!(stdout.contains("Stop"), "got: {stdout}");
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
    fs::write(project.join(SETTINGS), r#"{"hooks":{"Stop":[]},"x":1}"#).unwrap();

    assert!(manage("install", &project, &[]).status.success());
    assert!(manage("uninstall", &project, &[]).status.success());

    assert_eq!(settings(&project), serde_json::json!({"x": 1}));
}

#[test]
fn install_hooks_help_names_the_decision_prompt_stop_hook() {
    let output = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["manage", "install", "hooks", "--help"])
        .output()
        .expect("failed to run living-docs manage install hooks --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("decision-prompt"), "got: {stdout}");
}

#[test]
fn install_hooks_long_help_names_the_stop_wiring() {
    let output = Command::new(env!("CARGO_BIN_EXE_living-docs"))
        .args(["manage", "install", "hooks", "--help", "--plain"])
        .output()
        .expect("failed to run living-docs manage install hooks --help");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("`Stop`") || stdout.contains("Stop"),
        "got: {stdout}"
    );
}
