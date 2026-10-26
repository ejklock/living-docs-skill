//! The SessionStart hook prints one line that teaches the record reference
//! form, the `number` key of `set`, and the index repair after a merge.

use std::path::Path;
use std::process::Command;

fn hook_output() -> String {
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("skills/living-docs/hooks/session-context.sh");
    let output = Command::new("bash")
        .arg(&script)
        .env("CLAUDE_PROJECT_DIR", env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run the session-context script");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("hook output is utf-8")
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
    assert!(out.contains("`living-docs index` repairs"), "{out}");
    assert!(out.contains("after a merge"), "{out}");
}

#[test]
fn hook_still_names_every_verb_it_taught_before() {
    let out = hook_output();
    for verb in [
        "write ONLY the body below the closing ---",
        "living-docs new <type>",
        "living-docs set <TYPE/NNNN> <key> <value>",
        "`living-docs index`",
        "--body",
        "--from PATH|-",
        "section \"<heading>\"",
        "living-docs supersede",
        "living-docs fmt",
        "never argv",
        "fails `living-docs check` at commit",
    ] {
        assert!(out.contains(verb), "{verb}: {out}");
    }
}
