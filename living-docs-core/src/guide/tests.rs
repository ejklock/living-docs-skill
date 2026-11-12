use super::*;

#[test]
fn list_includes_every_embedded_skill_and_the_adr_topic() {
    let output = list().expect("list succeeds with an embedded corpus");
    assert!(output.contains("living-docs"));
    assert!(output.contains("okf-knowledge-format"));
    assert!(output.contains("research-artifacts"));
    assert!(output.contains("adr"));
}

#[test]
fn body_returns_the_skill_md_contents() {
    let output = body("living-docs").expect("living-docs/SKILL.md is embedded");
    assert!(output.contains("# Living Docs"));
}

#[test]
fn body_errors_for_an_unknown_skill() {
    assert!(body("no-such-skill").is_err());
}

#[test]
fn topic_adr_concatenates_the_rule_and_template() {
    let output = topic("living-docs", "adr").expect("adr topic resolves");
    assert!(output.contains("rules/adr-conventions.md"));
    assert!(output.contains("templates/adr.md"));
}

#[test]
fn topic_issue_workflow_resolves_only_the_rules_file() {
    let output = topic("living-docs", "issue-workflow").expect("issue-workflow topic resolves");
    assert!(output.contains("rules/issue-workflow.md"));
    assert!(!output.contains("templates/issue-workflow.md"));
}

#[test]
fn topic_errors_for_an_unknown_topic() {
    assert!(topic("living-docs", "no-such-topic").is_err());
}

#[test]
fn topic_errors_for_an_unknown_skill() {
    assert!(topic("no-such-skill", "adr").is_err());
}

#[test]
fn list_json_matches_list_names_and_topics() {
    let plain = list().expect("list succeeds with an embedded corpus");
    let output = list_json().expect("list_json succeeds with an embedded corpus");
    let parsed: SkillListJson = serde_json::from_str(&output).expect("list_json emits valid JSON");
    let living_docs = parsed
        .skills
        .iter()
        .find(|skill| skill.name == "living-docs")
        .expect("living-docs is present in list_json");
    assert!(living_docs.topics.contains(&"adr".to_owned()));
    assert!(plain.contains("living-docs"));
    assert!(!output.contains('\n'));
}

#[test]
fn body_json_matches_body_content() {
    let plain = body("living-docs").expect("living-docs/SKILL.md is embedded");
    let output = body_json("living-docs").expect("body_json succeeds for a known skill");
    let parsed: SkillBodyJson = serde_json::from_str(&output).expect("body_json emits valid JSON");
    assert_eq!(parsed.skill, "living-docs");
    assert_eq!(parsed.content, plain);
    assert!(!output.contains('\n'));
}

#[test]
fn body_json_errors_for_an_unknown_skill() {
    assert!(body_json("no-such-skill").is_err());
}

#[test]
fn topic_json_adr_matches_the_rule_and_template_ordering() {
    let output = topic_json("living-docs", "adr").expect("adr topic_json resolves");
    let parsed: SkillTopicJson =
        serde_json::from_str(&output).expect("topic_json emits valid JSON");
    assert_eq!(parsed.skill, "living-docs");
    assert_eq!(parsed.topic, "adr");
    let paths: Vec<&str> = parsed.parts.iter().map(|part| part.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "living-docs/rules/adr-conventions.md",
            "living-docs/templates/adr.md"
        ]
    );
    assert!(!output.contains('\n'));
}

#[test]
fn topic_json_errors_for_an_unknown_topic() {
    assert!(topic_json("living-docs", "no-such-topic").is_err());
}

#[test]
fn asset_returns_the_raw_bytes_of_an_embedded_hook_script() {
    let bytes =
        asset("living-docs/hooks/session-context.sh").expect("session-context.sh is embedded");
    assert!(bytes.starts_with(b"#!"));
}

#[test]
fn asset_returns_none_for_an_unknown_path() {
    assert!(asset("living-docs/hooks/no-such-script.sh").is_none());
}
