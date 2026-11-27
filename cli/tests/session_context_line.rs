//! The SessionStart hook prints one line that teaches the record reference
//! form, the `number` key of `set`, and the index repair after a merge.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A git command with the host's global and system configuration hidden.
fn isolated_git() -> Command {
    let mut cmd = Command::new("git");
    cmd.env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    cmd
}

/// A fresh git project under the target temporary directory, named for the
/// calling test so parallel tests never share one.
fn fresh_project(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create scratch project");
    let init = isolated_git()
        .args(["init", "-q"])
        .arg(&root)
        .status()
        .expect("git init");
    assert!(init.success());
    root
}

/// Runs the hook against `root` with the host git configuration hidden.
fn run_hook_in(root: &Path) -> std::process::Output {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/session-context.sh");
    Command::new("bash")
        .arg(&script)
        .env("CLAUDE_PROJECT_DIR", root)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("failed to run the session-context script")
}

fn hook_output() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = fresh_project(&format!("session-context-line-{id}"));
    let output = run_hook_in(&root);
    std::fs::remove_dir_all(&root).expect("remove scratch project");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("hook output is utf-8")
}

#[test]
fn hook_arms_the_githooks_directory_in_the_project_config_only() {
    let root = fresh_project("session-context-arms-githooks");
    std::fs::create_dir(root.join(".githooks")).unwrap();
    let output = run_hook_in(&root);
    assert!(output.status.success());
    let local = isolated_git()
        .arg("-C")
        .arg(&root)
        .args(["config", "--local", "core.hooksPath"])
        .output()
        .expect("git config");
    let value = String::from_utf8(local.stdout).unwrap();
    std::fs::remove_dir_all(&root).expect("remove scratch project");
    assert_eq!(value.trim(), ".githooks");
}

#[test]
fn hook_prints_a_single_line() {
    let out = hook_output();
    assert_eq!(out.trim_end_matches('\n').lines().count(), 1, "{out}");
}

#[test]
fn hook_teaches_the_type_nnnn_reference_form() {
    let out = hook_output();
    assert!(out.contains("TYPE/NNNN"), "{out}");
    assert!(out.contains("set adr/0001 status Accepted"), "{out}");
    assert!(!out.contains("set <NNNN>"), "{out}");
}

#[test]
fn hook_teaches_the_number_key_of_set() {
    let out = hook_output();
    assert!(out.contains("set <TYPE/NNNN> number next"), "{out}");
}

#[test]
fn hook_teaches_index_repairs_an_index_after_a_merge() {
    let out = hook_output();
    assert!(out.contains("`living-docs manage index` repairs"), "{out}");
    assert!(out.contains("after a merge"), "{out}");
}

#[test]
fn hook_teaches_reading_before_deciding_and_before_editing() {
    let out = hook_output();
    for read_form in [
        "living-docs read records --topic <term>",
        "read records --links-to <TYPE/NNNN>",
        "--impact <TYPE/NNNN>",
        "living-docs read guide <topic>",
    ] {
        assert!(out.contains(read_form), "{read_form}: {out}");
    }
}

#[test]
fn hook_names_the_index_as_what_a_hand_written_edit_breaks() {
    let out = hook_output();
    assert!(
        out.contains("or a hand edit. A hand-written index fails `living-docs check` at commit."),
        "{out}"
    );
}

#[test]
fn hook_exits_zero_and_stays_one_line_without_a_cli() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("session-context-no-cli");
    std::fs::create_dir_all(&root).expect("create scratch project");
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/session-context.sh");
    let output = Command::new("/bin/bash")
        .arg(&script)
        .env("CLAUDE_PROJECT_DIR", &root)
        .env("PATH", "/usr/bin:/bin")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("failed to run the session-context script");
    let out = String::from_utf8(output.stdout).expect("hook output is utf-8");
    assert!(output.status.success());
    assert!(out.contains("CLI not built"), "{out}");
    assert_eq!(out.trim_end_matches('\n').lines().count(), 1, "{out}");
    assert!(
        out.contains("living-docs read records --topic <term>"),
        "{out}"
    );
}

#[test]
fn hook_still_names_every_verb_it_taught_before() {
    let out = hook_output();
    for verb in [
        "write ONLY the body below the closing ---",
        "living-docs write new <type>",
        "living-docs write set <TYPE/NNNN> <key> <value>",
        "`living-docs manage index`",
        "--body",
        "--from PATH|-",
        "section \"<heading>\"",
        "living-docs write supersede",
        "living-docs manage fmt",
        "never argv",
        "fails `living-docs check` at commit",
    ] {
        assert!(out.contains(verb), "{verb}: {out}");
    }
}

#[test]
fn hook_teaches_recording_a_mid_work_choice_in_the_active_issues_decision_section() {
    let out = hook_output();
    for phrase in [
        "A choice made mid-work goes into the active issue's Decision section the moment you make it (`living-docs write set issue/NNNN section Decision --from -` replaces the whole section, so pass its current bullets plus the new one); write an ADR only when a future reader would pay to rediscover the choice.",
        "active issue's Decision section",
        "section Decision --from -",
        "only when a future reader would pay to rediscover",
    ] {
        assert!(out.contains(phrase), "{phrase}: {out}");
    }
    assert!(!out.contains("set <NNNN>"), "{out}");
    assert_eq!(out.trim_end_matches('\n').lines().count(), 1, "{out}");
}
