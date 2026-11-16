//! The decision reminder decides inside-bundle, outside-bundle and
//! outside-repository from the edited file's real path, whatever symlink or
//! working directory the session uses.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-C"])
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
}

struct Fixture {
    base: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("decision-paths-{name}"));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(repo.join("docs")).expect("docs");
        std::fs::create_dir_all(repo.join("sub/docs")).expect("sub docs");
        std::fs::create_dir_all(base.join("other")).expect("other");
        git(&repo, &["init", "-q", "-b", "main"]);
        std::fs::write(repo.join("docs/n.md"), "a\n").expect("seed");
        std::fs::write(repo.join("code.rs"), "fn a() {}\n").expect("seed");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "seed"]);
        std::fs::write(repo.join("code.rs"), "fn b() {}\n").expect("edit");
        std::os::unix::fs::symlink(&repo, base.join("link")).expect("symlink");
        Self { base }
    }

    fn at(&self, rel: &str) -> PathBuf {
        self.base.join(rel)
    }
}

fn run(cwd: &Path, file: &str) -> String {
    run_tool(cwd, file, "Edit", true)
}

fn run_tool(cwd: &Path, file: &str, tool: &str, payload_cwd: bool) -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/decision-prompt.sh");
    let mut payload = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "tool_name": tool,
        "tool_input": { "file_path": file },
    });
    if payload_cwd {
        payload["cwd"] = cwd.display().to_string().into();
    }
    let payload = payload.to_string();
    let mut child = Command::new("bash")
        .arg(script)
        .env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("LIVING_DOCS_BUNDLE")
        .env("LIVING_DOCS_DECISION_PROMPT", "1")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn hook");
    let mut pipe = child.stdin.take().unwrap();
    let _ = pipe.write_all(payload.as_bytes());
    drop(pipe);
    let out = child.wait_with_output().expect("hook runs");
    assert!(out.status.success(), "hook must exit 0");
    String::from_utf8(out.stdout).expect("utf-8")
}

fn assert_nudge(out: &str) {
    let value: serde_json::Value = serde_json::from_str(out).expect("output is valid JSON");
    assert_eq!(value["hookSpecificOutput"]["hookEventName"], "PostToolUse");
}

#[test]
fn symlinked_root_with_cwd_below_it_tells_code_from_bundle() {
    let fx = Fixture::new("symlink");
    let cwd = fx.at("link/sub");
    let bundle = fx.at("link/docs/n.md");
    assert_eq!(run(&cwd, bundle.to_str().unwrap()), "");
    let code = fx.at("link/sub/docs/x.rs");
    assert_nudge(&run(&cwd, code.to_str().unwrap()));
    assert_eq!(run(&cwd, code.to_str().unwrap()), "");
}

#[test]
fn relative_path_resolves_against_the_hook_cwd() {
    let fx = Fixture::new("relative");
    let cwd = fx.at("repo/sub");
    assert_eq!(run(&cwd, "../docs/n.md"), "");
    assert_nudge(&run(&cwd, "docs/x.rs"));
}

#[test]
fn path_outside_the_repository_never_nudges() {
    let fx = Fixture::new("outside");
    let cwd = fx.at("repo");
    let other = fx.at("other/x.rs");
    assert_eq!(run(&cwd, other.to_str().unwrap()), "");
    assert_eq!(run(&cwd, "../other/x.rs"), "");
    assert_eq!(run(&cwd, "/etc/hosts"), "");
}

#[test]
fn path_with_spaces_and_a_quote_nudges_with_valid_json() {
    let fx = Fixture::new("quote");
    let cwd = fx.at("repo");
    let file = fx.at("repo/sp ace/q\"t.rs");
    assert_nudge(&run(&cwd, file.to_str().unwrap()));
}

#[test]
fn write_of_a_new_file_in_a_new_directory_nudges() {
    let fx = Fixture::new("newdir");
    let cwd = fx.at("repo");
    let file = fx.at("repo/fresh/deep/x.rs");
    assert_nudge(&run_tool(&cwd, file.to_str().unwrap(), "Write", true));
}

#[test]
fn repository_path_holding_a_quote_is_read_from_the_escaped_payload() {
    let fx = Fixture::new("q\"uote");
    let cwd = fx.at("repo");
    let bundle = fx.at("repo/docs/n.md");
    assert_eq!(run_tool(&cwd, bundle.to_str().unwrap(), "Edit", false), "");
    let code = fx.at("repo/code.rs");
    assert_nudge(&run_tool(&cwd, code.to_str().unwrap(), "Edit", false));
    assert_eq!(run_tool(&cwd, code.to_str().unwrap(), "Edit", false), "");
}
