//! End-to-end wiring for the `guide` verb (ADR 0060, renamed from `skill`):
//! the positional-vs-`--skill` resolution, plain-text output, and the
//! retired `skill <name> --topic <t>` alias shape. JSON-shape coverage lives
//! in `cli/tests/guide_json.rs`, split out to stay under the file-size cap.

use std::process::{Command, Output};

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn run_guide(args: &[&str]) -> Output {
    living_docs()
        .arg("guide")
        .args(args)
        .output()
        .expect("failed to run living-docs guide")
}

fn run_skill_alias(args: &[&str]) -> Output {
    living_docs()
        .arg("skill")
        .args(args)
        .output()
        .expect("failed to run living-docs skill")
}

#[test]
fn list_prints_every_embedded_skill_and_the_adr_topic() {
    let output = run_guide(&["--list", "--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(stdout.contains("living-docs"), "got:\n{stdout}");
    assert!(stdout.contains("okf-knowledge-format"), "got:\n{stdout}");
    assert!(stdout.contains("research-artifacts"), "got:\n{stdout}");
    assert!(stdout.contains("adr"), "got:\n{stdout}");
    assert!(stdout.contains("code-comment-hygiene"), "got:\n{stdout}");
}

/// ADR 0019, AC ac-s4-3: the `living-docs` SKILL.md stub carries the exact
/// same body-only instruction `new` prints and the root `--help` about text
/// carries. `guide` with no positional resolves to the default skill
/// (`living-docs`) with no topic, printing the full body.
#[test]
fn no_args_prints_the_default_skills_full_body() {
    let output = run_guide(&["--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(
        stdout.contains(
            "Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs set` / `supersede`."
        ),
        "got:\n{stdout}"
    );
    assert!(stdout.contains("# Living Docs"), "got:\n{stdout}");
}

#[test]
fn positional_topic_prints_the_conventions_and_the_template_header() {
    let output = run_guide(&["adr", "--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(
        stdout.contains("captures **one** decision"),
        "expected adr-conventions.md content, got:\n{stdout}"
    );
    assert!(
        stdout.contains("templates/adr.md"),
        "expected the template header, got:\n{stdout}"
    );
}

#[test]
fn positional_matching_a_known_skill_prints_that_skills_body_with_no_topic() {
    let output = run_guide(&["okf-knowledge-format", "--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(
        stdout.contains("Represent knowledge as an **OKF bundle**"),
        "got:\n{stdout}"
    );
}

#[test]
fn explicit_skill_flag_treats_the_positional_as_a_topic() {
    let output = run_guide(&["conformance", "--skill", "okf-knowledge-format", "--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(stdout.contains("conformance"), "got:\n{stdout}");
}

#[test]
fn topic_code_comment_hygiene_prints_the_new_rule_and_no_template_header() {
    let output = run_guide(&["code-comment-hygiene", "--plain"]);
    let stdout = stdout_of(&output);
    let rule = "must never name or number a documentation artifact";
    let ok = stdout.contains("rules/code-comment-hygiene.md")
        && stdout.contains(rule)
        && !stdout.contains("templates/code-comment-hygiene.md");
    assert!(ok, "got:\n{stdout}");
}

#[test]
fn unknown_topic_exits_non_zero_with_a_stderr_message() {
    let output = run_guide(&["no-such-topic", "--plain"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected a failure exit code, got:\n{}",
        stdout_of(&output)
    );
    assert!(
        !stderr_of(&output).is_empty(),
        "expected a stderr message on unknown topic"
    );
}

#[test]
fn unknown_skill_flag_exits_non_zero_with_a_stderr_message() {
    let output = run_guide(&["--skill", "no-such-skill", "--plain"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected a failure exit code, got:\n{}",
        stdout_of(&output)
    );
    assert!(
        !stderr_of(&output).is_empty(),
        "expected a stderr message on unknown skill"
    );
}

#[test]
fn retired_skill_alias_keeps_its_old_name_then_topic_flag_shape() {
    let output = run_skill_alias(&["living-docs", "--topic", "adr", "--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(
        stdout.contains("captures **one** decision"),
        "got:\n{stdout}"
    );
    assert!(stdout.contains("templates/adr.md"), "got:\n{stdout}");
}

#[test]
fn skill_alias_is_hidden_from_help() {
    let help = stdout_of(
        &living_docs()
            .arg("--help")
            .output()
            .expect("failed to run living-docs --help"),
    );
    let lists_skill_as_a_command = help
        .lines()
        .any(|line| line.trim() == "skill" || line.trim_start().starts_with("skill  "));
    assert!(
        !lists_skill_as_a_command,
        "hidden alias must not be listed as a command in help:\n{help}"
    );
    assert!(
        help.lines()
            .any(|line| line.trim_start().starts_with("guide")),
        "guide must appear in help:\n{help}"
    );
}
