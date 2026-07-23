use serde::Deserialize;
use std::process::{Command, Output};

#[derive(Deserialize)]
struct SkillListJson {
    skills: Vec<SkillSummaryJson>,
}

#[derive(Deserialize)]
struct SkillSummaryJson {
    name: String,
    topics: Vec<String>,
}

#[derive(Deserialize)]
struct SkillBodyJson {
    skill: String,
    content: String,
}

#[derive(Deserialize)]
struct SkillTopicJson {
    #[allow(dead_code)]
    skill: String,
    #[allow(dead_code)]
    topic: String,
    parts: Vec<TopicPartJson>,
}

#[derive(Deserialize)]
struct TopicPartJson {
    path: String,
    #[allow(dead_code)]
    content: String,
}

fn living_docs() -> Command {
    Command::new(env!("CARGO_BIN_EXE_living-docs"))
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn run_skill(args: &[&str]) -> Output {
    living_docs()
        .arg("skill")
        .args(args)
        .output()
        .expect("failed to run living-docs skill")
}

#[test]
fn list_prints_every_embedded_skill_and_the_adr_topic() {
    let output = run_skill(&["--list", "--plain"]);
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

/// ADR 0019, AC ac-s4-3: the `living-docs` SKILL.md stub carries the exact
/// same body-only instruction `new` prints and the root `--help` about text
/// carries.
#[test]
fn body_carries_the_body_only_instruction_matching_new_and_help() {
    let output = run_skill(&["living-docs", "--plain"]);
    let stdout = stdout_of(&output);

    assert!(
        stdout.contains(
            "Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs status` / `supersede` / `index`."
        ),
        "got:\n{stdout}"
    );
}

#[test]
fn body_prints_the_skill_md_h1() {
    let output = run_skill(&["living-docs", "--plain"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert!(stdout.contains("# Living Docs"), "got:\n{stdout}");
}

#[test]
fn topic_adr_prints_the_conventions_and_the_template_header() {
    let output = run_skill(&["living-docs", "--topic", "adr", "--plain"]);
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
fn unknown_topic_exits_non_zero_with_a_stderr_message() {
    let output = run_skill(&["living-docs", "--topic", "no-such-topic", "--plain"]);
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
fn unknown_skill_exits_non_zero_with_a_stderr_message() {
    let output = run_skill(&["no-such-skill", "--plain"]);
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
fn missing_name_without_list_exits_non_zero() {
    let output = run_skill(&["--plain"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected a failure exit code when NAME and --list are both absent"
    );
}

#[test]
fn list_json_prints_minified_single_line_json_with_the_adr_topic() {
    let output = run_skill(&["--list", "--json"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "expected a single line of output, got:\n{stdout}"
    );
    let parsed: SkillListJson =
        serde_json::from_str(stdout.trim_end()).expect("--list --json emits valid JSON");
    let living_docs = parsed
        .skills
        .iter()
        .find(|skill| skill.name == "living-docs")
        .expect("living-docs is present in --list --json output");
    assert!(
        living_docs.topics.contains(&"adr".to_owned()),
        "expected the adr topic, got: {:?}",
        living_docs.topics
    );
}

#[test]
fn body_json_prints_the_skill_md_content_as_json() {
    let output = run_skill(&["living-docs", "--json"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    let parsed: SkillBodyJson =
        serde_json::from_str(stdout.trim_end()).expect("skill --json emits valid JSON");
    assert_eq!(parsed.skill, "living-docs");
    assert!(
        parsed.content.contains("# Living Docs"),
        "got: {}",
        parsed.content
    );
}

#[test]
fn topic_json_adr_lists_the_conventions_and_template_paths() {
    let output = run_skill(&["living-docs", "--topic", "adr", "--json"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    let parsed: SkillTopicJson =
        serde_json::from_str(stdout.trim_end()).expect("--topic --json emits valid JSON");
    let paths: Vec<&str> = parsed.parts.iter().map(|part| part.path.as_str()).collect();
    assert!(
        paths.contains(&"living-docs/templates/adr.md"),
        "got: {paths:?}"
    );
    assert_eq!(
        paths
            .iter()
            .filter(|path| **path == "living-docs/rules/adr-conventions.md")
            .count(),
        1,
        "got: {paths:?}"
    );
}

#[test]
fn unknown_topic_with_json_exits_non_zero_with_a_stderr_message() {
    let output = run_skill(&["living-docs", "--topic", "no-such-topic", "--json"]);
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
fn no_flag_with_piped_stdout_defaults_to_json() {
    let output = run_skill(&["--list"]);
    let stdout = stdout_of(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success, got:\n{stdout}\n{}",
        stderr_of(&output)
    );
    let parsed: SkillListJson = serde_json::from_str(stdout.trim_end())
        .expect("piped stdout with no flag defaults to JSON");
    assert!(
        parsed
            .skills
            .iter()
            .any(|skill| skill.name == "living-docs"),
        "expected living-docs in the default-JSON output, got:\n{stdout}"
    );
}

#[test]
fn json_and_plain_together_is_a_usage_error() {
    let output = run_skill(&["--list", "--json", "--plain"]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected a clap conflict failure, got:\n{}",
        stdout_of(&output)
    );
    assert!(
        !stderr_of(&output).is_empty(),
        "expected clap's usage-error message on stderr"
    );
}
