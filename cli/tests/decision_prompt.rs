//! The opt-in PostToolUse hook adds a non-blocking reminder once per commit
//! cycle, stays silent unless the project opts in, and keeps its state under
//! the git directory.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-C"])
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).expect("utf-8")
}

fn repo(name: &str, branch: &str) -> PathBuf {
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

fn run_with(dir: &Path, opt_in: bool, stdin: &str, bundle: Option<&str>) -> String {
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
    let mut child = cmd
        .current_dir(dir)
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

fn tool_json(dir: &Path, tool: &str, file: &str) -> String {
    format!(
        r#"{{"hook_event_name":"PostToolUse","session_id":"s","tool_name":"{tool}","tool_input":{{"file_path":"{}"}},"cwd":"{}"}}"#,
        dir.join(file).display(),
        dir.display()
    )
}

fn edit_json(dir: &Path) -> String {
    tool_json(dir, "Edit", "code.rs")
}

fn run(dir: &Path) -> String {
    run_with(dir, true, &edit_json(dir), None)
}

fn context(out: &str) -> String {
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

struct Cleanup(PathBuf);

impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn edit_code(dir: &Path, text: &str) {
    std::fs::write(dir.join("code.rs"), text).expect("edit");
}

#[test]
fn hook_is_silent_without_the_opt_in_in_every_state() {
    let dir = repo("off", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run_with(&dir, false, &edit_json(&dir), None), "");
    assert_eq!(run_with(&dir, false, "garbage", None), "");
}

#[test]
fn hook_adds_the_materiality_reminder_once() {
    let dir = repo("block", "main");
    edit_code(&dir, "fn b() {}\n");
    let out = run(&dir);
    let reason = context(&out);
    assert!(
        reason.contains("active issue's Decision section"),
        "{reason}"
    );
    assert!(reason.ends_with("otherwise carry on."), "{reason}");
    assert!(!reason.contains("and stop"), "{reason}");
    assert!(reason.contains("replaces the whole section"), "{reason}");
    assert!(
        reason.contains("only when a future reader would pay to rediscover"),
        "{reason}"
    );
}

#[test]
fn hook_stays_silent_when_a_bundle_file_also_changed() {
    let dir = repo("bundle", "main");
    edit_code(&dir, "fn b() {}\n");
    std::fs::write(dir.join("docs/note.md"), "b\n").expect("edit");
    assert_eq!(run(&dir), "");
}

#[test]
fn hook_stays_silent_for_an_edit_of_a_bundle_file() {
    let dir = repo("bundleedit", "main");
    edit_code(&dir, "fn b() {}\n");
    let json = tool_json(&dir, "Write", "docs/note.md");
    assert_eq!(run_with(&dir, true, &json, None), "");
    let relative = r#"{"tool_name":"Edit","tool_input":{"file_path":"docs/x.md"},"cwd":"CWD"}"#;
    let relative = relative.replace("CWD", &dir.display().to_string());
    assert_eq!(run_with(&dir, true, &relative, None), "");
}

#[test]
fn hook_stays_silent_for_tools_that_do_not_edit_files() {
    let dir = repo("read", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(
        run_with(&dir, true, &tool_json(&dir, "Read", "code.rs"), None),
        ""
    );
    assert_eq!(
        run_with(&dir, true, &tool_json(&dir, "Bash", "code.rs"), None),
        ""
    );
    assert!(
        run_with(&dir, true, &tool_json(&dir, "MultiEdit", "code.rs"), None)
            .contains("PostToolUse")
    );
}

#[test]
fn hook_stays_silent_for_the_same_change_set_a_second_time() {
    let dir = repo("twice", "main");
    edit_code(&dir, "fn b() {}\n");
    assert!(run(&dir).contains("additionalContext"));
    assert_eq!(run(&dir), "");
}

#[test]
fn hook_stays_silent_after_further_edits_and_new_files_on_the_same_head() {
    let dir = repo("again", "main");
    edit_code(&dir, "fn b() {}\n");
    assert!(run(&dir).contains("additionalContext"));
    edit_code(&dir, "fn c() {}\n");
    assert_eq!(run(&dir), "");
    std::fs::write(dir.join("extra.rs"), "fn e() {}\n").expect("new file");
    assert_eq!(run(&dir), "");
}

/// A commit starts a new cycle: the next code edit prompts once, then stays silent.
#[test]
fn hook_reminds_once_more_after_a_commit_then_edit() {
    let dir = repo("cycle", "main");
    edit_code(&dir, "fn b() {}\n");
    assert!(run(&dir).contains("additionalContext"));
    git(&dir, &["commit", "-q", "-am", "work"]);
    assert_eq!(run(&dir), "");
    edit_code(&dir, "fn c() {}\n");
    assert!(run(&dir).contains("additionalContext"));
    edit_code(&dir, "fn d() {}\n");
    assert_eq!(run(&dir), "");
}

#[test]
fn hook_prompts_once_in_a_repository_without_commits_until_its_first_commit() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("decision-prompt-nocommit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create repo");
    git(&dir, &["init", "-q", "-b", "main"]);
    edit_code(&dir, "fn a() {}\n");
    assert!(run(&dir).contains("additionalContext"));
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run(&dir), "");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "first"]);
    edit_code(&dir, "fn c() {}\n");
    assert!(run(&dir).contains("additionalContext"));
}

#[test]
fn hook_stays_silent_outside_a_git_repository_and_on_malformed_stdin() {
    let plain = std::env::temp_dir().join(format!("decision-prompt-plain-{}", std::process::id()));
    std::fs::create_dir_all(&plain).expect("create dir");
    let _cleanup = Cleanup(plain.clone());
    assert_eq!(run_with(&plain, true, &edit_json(&plain), None), "");
    let dir = repo("malformed", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run_with(&dir, true, "not json", None), "");
    assert_eq!(run_with(&dir, true, "", None), "");
}

#[test]
fn hook_stays_silent_for_json_without_a_tool_name_and_path() {
    let dir = repo("nokey", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run_with(&dir, true, "{garbage}", None), "");
    assert_eq!(run_with(&dir, true, r#"{"cwd":"x"}"#, None), "");
    let no_path = format!(r#"{{"tool_name":"Edit","cwd":"{}"}}"#, dir.display());
    assert_eq!(run_with(&dir, true, &no_path, None), "");
}

#[test]
fn hook_stays_silent_when_a_file_is_renamed_out_of_the_bundle() {
    let dir = repo("renameout", "main");
    git(&dir, &["mv", "docs/note.md", "n.md"]);
    assert_eq!(run(&dir), "");
}

#[test]
fn reason_names_the_issue_carried_by_the_branch() {
    let dir = repo("branch", "task/ek-0121-some-work");
    edit_code(&dir, "fn b() {}\n");
    let out = run(&dir);
    assert!(out.contains("issue/0121"), "{out}");
    assert!(!out.contains("issue/NNNN"), "{out}");
}

#[test]
fn reason_prefers_the_number_after_the_last_slash_of_the_branch() {
    let dir = repo("release", "release-2026/0121-x");
    edit_code(&dir, "fn b() {}\n");
    let reason = context(&run(&dir));
    assert!(reason.contains("issue/0121"), "{reason}");
    assert!(!reason.contains("issue/2026"), "{reason}");
}

#[test]
fn reason_prefers_the_number_after_a_dash_over_an_earlier_year() {
    let dir = repo("dashnum", "feat/2026-0121");
    edit_code(&dir, "fn b() {}\n");
    let reason = context(&run(&dir));
    assert!(reason.contains("issue/0121"), "{reason}");
    assert!(!reason.contains("issue/2026"), "{reason}");
}

#[test]
fn reason_uses_the_placeholder_on_a_branch_without_a_number() {
    let dir = repo("nonumber", "main");
    edit_code(&dir, "fn b() {}\n");
    let out = run(&dir);
    assert!(out.contains("issue/NNNN"), "{out}");
    assert!(!out.contains("issue/0"), "{out}");
}

#[test]
fn state_lives_under_the_git_directory_not_in_the_tree() {
    let dir = repo("state", "main");
    edit_code(&dir, "fn b() {}\n");
    let before = git(&dir, &["status", "--porcelain"]);
    assert!(run(&dir).contains("additionalContext"));
    assert_eq!(git(&dir, &["status", "--porcelain"]), before);
}

#[test]
fn bundle_variable_moves_the_bundle_directory() {
    let dir = repo("handbook", "main");
    std::fs::create_dir_all(dir.join("handbook")).expect("mkdir");
    edit_code(&dir, "fn b() {}\n");
    std::fs::write(dir.join("handbook/x.md"), "x\n").expect("write");
    let json = tool_json(&dir, "Edit", "handbook/x.md");
    assert_eq!(run_with(&dir, true, &json, Some("handbook")), "");
    assert!(run_with(&dir, true, &json, None).contains("additionalContext"));
}
