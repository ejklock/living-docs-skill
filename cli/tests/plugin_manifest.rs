//! The repository root is a Claude Code plugin and a marketplace listing it:
//! the manifests declare the MCP server and the session hook inline.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_json(relative: &str) -> Value {
    let text = std::fs::read_to_string(repo_root().join(relative))
        .unwrap_or_else(|e| panic!("read {relative}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {relative}: {e}"))
}

fn version() -> String {
    std::fs::read_to_string(repo_root().join("VERSION"))
        .expect("read VERSION")
        .trim()
        .to_string()
}

fn plugin() -> Value {
    read_json(".claude-plugin/plugin.json")
}

/// The single SessionStart command, after checking it is the only hook.
fn session_start_command(plugin: &Value) -> String {
    let events = plugin["hooks"].as_object().expect("hooks is an object");
    assert_eq!(events.keys().collect::<Vec<_>>(), ["SessionStart"]);
    let groups = events["SessionStart"].as_array().expect("event groups");
    assert_eq!(groups.len(), 1);
    let hooks = groups[0]["hooks"].as_array().expect("group hooks");
    assert_eq!(hooks.len(), 1);
    assert_eq!(hooks[0]["type"], "command");
    hooks[0]["command"].as_str().expect("command").to_string()
}

fn run_command(command: &str, project: &Path) -> String {
    let output = Command::new("bash")
        .arg("-c")
        .arg(command)
        .env_remove("LIVING_DOCS_PLUGIN")
        .env_remove("LIVING_DOCS_BUNDLE")
        .env("CLAUDE_PLUGIN_ROOT", repo_root())
        .env("CLAUDE_PROJECT_DIR", project)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("run the hook command");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("utf-8 output")
}

fn project(name: &str, index: Option<&str>) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::create_dir(root.join(".githooks")).unwrap();
    let init = Command::new("git")
        .args(["init", "-q"])
        .arg(&root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git init");
    assert!(init.success());
    if let Some(text) = index {
        std::fs::write(root.join("docs/index.md"), text).unwrap();
    }
    root
}

fn hooks_path(root: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "--local", "core.hooksPath"])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git config");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn tracked(pathspec: &str) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root())
        .args(["ls-files", "--", pathspec])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git ls-files");
    assert!(
        output.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn plugin_version_equals_the_version_file() {
    assert_eq!(plugin()["version"], version().as_str());
}

#[test]
fn plugin_runs_the_mcp_server_through_npx_pinned_to_the_version() {
    let server = &plugin()["mcpServers"]["living-docs"];
    let pinned = format!("@ejklock/living-docs@{}", version());
    assert_eq!(server["command"], "npx");
    assert_eq!(
        server["args"],
        serde_json::json!([
            "-y",
            pinned,
            "mcp",
            "--docs-dir",
            "${CLAUDE_PROJECT_DIR}/docs"
        ])
    );
}

#[test]
fn session_hook_prints_the_session_line_in_a_marked_bundle() {
    let command = session_start_command(&plugin());
    let marked = "---\nokf_version: \"0.1\"\n---\n\n# Docs\n";
    let out = run_command(&command, &project("manifest-hook-marked", Some(marked)));
    assert!(out.contains("living-docs: "), "{out}");
}

#[test]
fn session_hook_is_silent_without_a_bundle_marker() {
    let command = session_start_command(&plugin());
    let out = run_command(&command, &project("manifest-hook-unmarked", None));
    assert_eq!(out, "");
}

#[test]
fn session_hook_leaves_the_git_hooks_path_unset() {
    let command = session_start_command(&plugin());
    let marked = "---\nokf_version: \"0.1\"\n---\n\n# Docs\n";
    let root = project("manifest-hook-git-config", Some(marked));
    run_command(&command, &root);
    assert_eq!(hooks_path(&root), "");
}

#[test]
fn marketplace_lists_only_the_plugin_by_its_manifest_name() {
    let marketplace = read_json(".claude-plugin/marketplace.json");
    let entries = marketplace["plugins"].as_array().expect("plugins");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["name"], plugin()["name"]);
    assert_eq!(entries[0]["name"], "living-docs");
    assert_eq!(entries[0]["source"], "./");
}

#[test]
fn root_tracks_no_local_mcp_config_and_no_hooks_file() {
    assert_eq!(tracked(".mcp.json"), "");
    assert_eq!(tracked("hooks/hooks.json"), "");
}
