//! The opt-in Stop hook blocks once per distinct set of code changes, stays
//! silent unless the project opts in, and keeps its state under the git directory.

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

fn stop_json(dir: &Path, active: bool) -> String {
    format!(
        r#"{{"session_id":"s","stop_hook_active":{active},"cwd":"{}"}}"#,
        dir.display()
    )
}

fn run(dir: &Path) -> String {
    run_with(dir, true, &stop_json(dir, false), None)
}

fn block_reason(out: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(out).expect("output is valid JSON");
    assert_eq!(value["decision"], "block", "{out}");
    value["reason"]
        .as_str()
        .expect("reason is a string")
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
    assert_eq!(run_with(&dir, false, &stop_json(&dir, false), None), "");
    assert_eq!(run_with(&dir, false, "garbage", None), "");
    assert_eq!(run_with(&dir, false, &stop_json(&dir, true), None), "");
}

#[test]
fn hook_blocks_once_with_the_materiality_message() {
    let dir = repo("block", "main");
    edit_code(&dir, "fn b() {}\n");
    let out = run(&dir);
    let reason = block_reason(&out);
    assert!(
        reason.contains("active issue's Decision section"),
        "{reason}"
    );
    assert!(reason.contains("no material decision"), "{reason}");
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
fn hook_stays_silent_when_a_stop_hook_already_continued() {
    let dir = repo("active", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run_with(&dir, true, &stop_json(&dir, true), None), "");
}

#[test]
fn hook_stays_silent_for_the_same_change_set_a_second_time() {
    let dir = repo("twice", "main");
    edit_code(&dir, "fn b() {}\n");
    assert!(run(&dir).contains("block"));
    assert_eq!(run(&dir), "");
}

#[test]
fn hook_blocks_again_after_a_further_edit_to_a_changed_file() {
    let dir = repo("again", "main");
    edit_code(&dir, "fn b() {}\n");
    assert!(run(&dir).contains("block"));
    edit_code(&dir, "fn c() {}\n");
    assert!(run(&dir).contains("block"));
}

#[test]
fn hook_stays_silent_outside_a_git_repository_and_on_malformed_stdin() {
    let plain = std::env::temp_dir().join(format!("decision-prompt-plain-{}", std::process::id()));
    std::fs::create_dir_all(&plain).expect("create dir");
    let _cleanup = Cleanup(plain.clone());
    assert_eq!(run_with(&plain, true, &stop_json(&plain, false), None), "");
    let dir = repo("malformed", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run_with(&dir, true, "not json", None), "");
    assert_eq!(run_with(&dir, true, "", None), "");
}

#[test]
fn hook_stays_silent_for_json_without_the_stop_hook_active_key() {
    let dir = repo("nokey", "main");
    edit_code(&dir, "fn b() {}\n");
    assert_eq!(run_with(&dir, true, "{garbage}", None), "");
    assert_eq!(run_with(&dir, true, r#"{"cwd":"x"}"#, None), "");
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
    let reason = block_reason(&run(&dir));
    assert!(reason.contains("issue/0121"), "{reason}");
    assert!(!reason.contains("issue/2026"), "{reason}");
}

#[test]
fn reason_prefers_the_number_after_a_dash_over_an_earlier_year() {
    let dir = repo("dashnum", "feat/2026-0121");
    edit_code(&dir, "fn b() {}\n");
    let reason = block_reason(&run(&dir));
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
    assert!(run(&dir).contains("block"));
    assert_eq!(git(&dir, &["status", "--porcelain"]), before);
}

#[test]
fn bundle_variable_moves_the_bundle_directory() {
    let dir = repo("handbook", "main");
    std::fs::create_dir_all(dir.join("handbook")).expect("mkdir");
    edit_code(&dir, "fn b() {}\n");
    std::fs::write(dir.join("handbook/x.md"), "x\n").expect("write");
    let json = stop_json(&dir, false);
    assert_eq!(run_with(&dir, true, &json, Some("handbook")), "");
    assert!(run_with(&dir, true, &json, None).contains("block"));
}
