use crate::common::{call, connect, data, is_error, message, Era, Scratch};
use rmcp::model::CallToolResult;
use serde_json::{json, Value};
use std::fs;

fn on_disk(relative: &str) -> String {
    let path = format!("{}/../skills/{relative}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

async fn guide(arguments: Value) -> CallToolResult {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let mut args = json!({"operation": "guide"});
    args.as_object_mut()
        .unwrap()
        .extend(arguments.as_object().unwrap().clone());
    call(&client, "read", args).await
}

fn paths(parts: &Value) -> Vec<String> {
    parts
        .as_array()
        .expect("parts")
        .iter()
        .filter_map(|part| part["path"].as_str().map(str::to_string))
        .collect()
}

#[tokio::test]
async fn guide_list_names_the_skills_in_sorted_order_with_their_topics() {
    let result = guide(json!({"list": true})).await;
    assert!(!is_error(&result), "{}", message(&result));
    let listing = data(&result);
    let skills = listing["skills"].as_array().expect("skills");
    let names: Vec<&str> = skills.iter().filter_map(|s| s["name"].as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    for skill in ["living-docs", "okf-knowledge-format", "research-artifacts"] {
        assert!(names.contains(&skill), "{names:?}");
    }
    let topics: Vec<&str> = skills[0]["topics"]
        .as_array()
        .expect("topics")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(topics.contains(&"adr") && topics.contains(&"issue-workflow"));
}

#[tokio::test]
async fn guide_without_arguments_returns_the_default_skill_body() {
    let result = guide(json!({})).await;
    assert!(!is_error(&result), "{}", message(&result));
    let body = data(&result);
    assert_eq!(body["skill"], "living-docs");
    assert_eq!(body["content"], on_disk("living-docs/SKILL.md"));
}

#[tokio::test]
async fn guide_with_a_skill_returns_that_skill_body() {
    let result = guide(json!({"skill": "okf-knowledge-format"})).await;
    assert!(!is_error(&result), "{}", message(&result));
    let body = data(&result);
    assert_eq!(body["skill"], "okf-knowledge-format");
    assert_eq!(body["content"], on_disk("okf-knowledge-format/SKILL.md"));
}

#[tokio::test]
async fn guide_topic_returns_the_rule_then_the_template() {
    let result = guide(json!({"topic": "adr"})).await;
    assert!(!is_error(&result), "{}", message(&result));
    let topic = data(&result);
    assert_eq!(
        paths(&topic["parts"]),
        [
            "living-docs/rules/adr-conventions.md",
            "living-docs/templates/adr.md"
        ]
    );
    for part in topic["parts"].as_array().expect("parts") {
        assert_eq!(part["content"], on_disk(part["path"].as_str().unwrap()));
    }
}

#[tokio::test]
async fn guide_topic_without_a_template_returns_only_the_rules_part() {
    let result = guide(json!({"topic": "issue-workflow"})).await;
    assert!(!is_error(&result), "{}", message(&result));
    assert_eq!(
        paths(&data(&result)["parts"]),
        ["living-docs/rules/issue-workflow.md"]
    );
}

#[tokio::test]
async fn guide_names_an_unknown_topic_and_an_unknown_skill() {
    let topic = guide(json!({"topic": "no-such-topic"})).await;
    assert!(is_error(&topic));
    assert!(
        message(&topic).contains("no-such-topic"),
        "{}",
        message(&topic)
    );
    let skill = guide(json!({"skill": "no-such-skill"})).await;
    assert!(is_error(&skill));
    assert!(
        message(&skill).contains("no-such-skill"),
        "{}",
        message(&skill)
    );
}

#[tokio::test]
async fn guide_list_refuses_a_skill_or_a_topic() {
    for extra in [json!({"skill": "living-docs"}), json!({"topic": "adr"})] {
        let mut args = json!({"list": true});
        args.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let result = guide(args).await;
        assert!(is_error(&result), "{}", message(&result));
        assert!(message(&result).contains("list"), "{}", message(&result));
    }
}

#[tokio::test]
async fn read_schema_and_instructions_name_the_guide_operation() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let tools = client.list_all_tools().await.expect("tools/list");
    let read = tools.iter().find(|tool| tool.name == "read").expect("read");
    let schema = Value::Object((*read.input_schema).clone());
    assert!(schema.to_string().contains("\"guide\""), "{schema}");
    let info = client.peer_info().expect("discover result");
    let instructions = info.instructions.clone().expect("instructions");
    assert!(instructions.contains("guide"), "{instructions}");
}
