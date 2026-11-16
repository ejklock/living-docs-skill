//! Shared helpers for the decision-prompt hook tests.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-C"])
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).expect("utf-8")
}

pub fn repo(name: &str, branch: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("decision-prompt-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs")).expect("create repo");
    git(&dir, &["init", "-q", "-b", branch]);
    std::fs::write(dir.join("docs/note.md"), "a\n").expect("seed");
    std::fs::write(dir.join("code.rs"), "fn a() {}\n").expect("seed");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "seed"]);
    dir
}

fn hook_command(opt_in: bool, bundle: Option<&str>) -> Command {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/decision-prompt.sh");
    let mut cmd = Command::new("bash");
    cmd.arg(script)
        .env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("LIVING_DOCS_DECISION_PROMPT")
        .env_remove("LIVING_DOCS_BUNDLE");
    if opt_in {
        cmd.env("LIVING_DOCS_DECISION_PROMPT", "1");
    }
    if let Some(bundle) = bundle {
        cmd.env("LIVING_DOCS_BUNDLE", bundle);
    }
    cmd
}

fn feed(mut cmd: Command, start: &Path, stdin: &str) -> String {
    let mut child = cmd
        .current_dir(start)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn hook");
    let mut pipe = child.stdin.take().unwrap();
    let _ = pipe.write_all(stdin.as_bytes());
    drop(pipe);
    let out = child.wait_with_output().expect("hook runs");
    assert!(out.status.success(), "hook must exit 0");
    String::from_utf8(out.stdout).expect("utf-8")
}

pub fn run_with(dir: &Path, opt_in: bool, stdin: &str, bundle: Option<&str>) -> String {
    feed(hook_command(opt_in, bundle), dir, stdin)
}

/// Runs the opted-in hook from `start`, an unrelated directory, with
/// `CLAUDE_PROJECT_DIR` set to `project_dir` when given, so only the payload's
/// `cwd` can lead it to a repository.
pub fn run_from(start: &Path, project_dir: Option<&Path>, stdin: &str) -> String {
    let mut cmd = hook_command(true, None);
    if let Some(project_dir) = project_dir {
        cmd.env("CLAUDE_PROJECT_DIR", project_dir);
    }
    feed(cmd, start, stdin)
}

pub fn tool_json(dir: &Path, tool: &str, file: &str) -> String {
    format!(
        r#"{{"hook_event_name":"PostToolUse","session_id":"s","tool_name":"{tool}","tool_input":{{"file_path":"{}"}},"cwd":"{}"}}"#,
        dir.join(file).display(),
        dir.display()
    )
}

pub fn edit_json(dir: &Path) -> String {
    tool_json(dir, "Edit", "code.rs")
}

pub fn run(dir: &Path) -> String {
    run_with(dir, true, &edit_json(dir), None)
}

pub fn context(out: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(out).expect("output is valid JSON");
    let object = value.as_object().expect("output is an object");
    assert!(!object.contains_key("decision"), "{out}");
    assert!(!object.contains_key("continue"), "{out}");
    assert_eq!(object.len(), 1, "{out}");
    let specific = &value["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], "PostToolUse", "{out}");
    specific["additionalContext"]
        .as_str()
        .expect("additionalContext is a string")
        .to_string()
}

pub struct Cleanup(pub PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn edit_code(dir: &Path, text: &str) {
    std::fs::write(dir.join("code.rs"), text).expect("edit");
}
