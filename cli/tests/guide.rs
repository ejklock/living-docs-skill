//! End-to-end wiring for the `guide` verb: the positional-vs-`--skill`
//! resolution, plain-text output, and the refusal of the retired `skill`
//! command and `--topic` flag. JSON-shape coverage lives
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

fn run_root(args: &[&str]) -> Output {
    living_docs()
        .args(args)
        .output()
        .expect("failed to run living-docs")
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
}

/// The `living-docs` SKILL.md stub carries the exact
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
fn removed_topics_are_gone_and_kept_index_topics_still_print() {
    // Joined from parts so the repository-wide absence grep stays empty.
    let gone = [
        "glossary".to_owned(),
        ["architecture", "index"].join("-"),
        ["code", "comment", "hygiene"].join("-"),
    ];
    let list = stdout_of(&run_guide(&["--list", "--plain"]));
    for topic in &gone {
        assert!(!list.contains(topic), "list still names {topic}:\n{list}");
        let output = run_guide(&[topic, "--plain"]);
        assert_eq!(output.status.code(), Some(2), "topic {topic}");
    }
    for topic in ["semantic-index", "context-index"] {
        let output = run_guide(&[topic, "--plain"]);
        assert_eq!(output.status.code(), Some(0), "topic {topic}");
        assert!(!stdout_of(&output).trim().is_empty(), "topic {topic}");
    }
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
fn retired_skill_command_is_refused_as_unknown_usage() {
    let output = run_root(&["skill", "living-docs"]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
    assert!(stderr_of(&output).contains("unrecognized subcommand"));
}

#[test]
fn retired_guide_topic_flag_is_refused_as_unknown_usage() {
    let output = run_guide(&["--topic", "adr", "--plain"]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
    assert!(stderr_of(&output).contains("unexpected argument"));
}

#[test]
fn kept_guide_spellings_still_work_after_the_refusals() {
    let topic = run_guide(&["adr", "--plain"]);
    assert_eq!(topic.status.code(), Some(0), "{}", stderr_of(&topic));
    assert!(stdout_of(&topic).contains("captures **one** decision"));
    let skill = run_guide(&["--skill", "living-docs", "--plain"]);
    assert_eq!(skill.status.code(), Some(0), "{}", stderr_of(&skill));
    let body = stdout_of(&skill);
    assert!(body.contains("# Living Docs"), "got:\n{body}");
}
