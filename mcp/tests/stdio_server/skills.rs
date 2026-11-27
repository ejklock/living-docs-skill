use crate::common::{connect, Client, Era, Scratch};
use rmcp::model::{
    GetSkillRequestParams, ReadResourceRequestParams, ResourceContents, Skill, SkillResources,
};
use rmcp::service::ServiceError;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

/// What each skill ships, written out independently of the server's table.
const INSTALLABLE: [(&str, &[&str]); 4] = [
    ("living-docs", &["SKILL.md"]),
    (
        "living-docs-drift",
        &["SKILL.md", "scripts/drift.sh", "prompt.md"],
    ),
    (
        "okf-knowledge-format",
        &["SKILL.md", "reference/SPEC.md", "reference/SPEC.source.md"],
    ),
    ("research-artifacts", &["SKILL.md"]),
];

fn corpus_bytes(skill: &str, path: &str) -> Vec<u8> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills");
    fs::read(root.join(skill).join(path)).expect("corpus file")
}

fn uri(skill: &str, path: &str) -> String {
    format!("skill://{skill}/{path}")
}

fn expected_frontmatter(name: &str) -> Value {
    let md = String::from_utf8(corpus_bytes(name, "SKILL.md")).expect("utf-8");
    let block = md
        .strip_prefix("---\n")
        .and_then(|rest| rest.split("\n---\n").next());
    serde_yaml::from_str(block.expect("frontmatter")).expect("yaml")
}

fn expected_files(name: &str, paths: &[&str]) -> Vec<Value> {
    paths
        .iter()
        .map(|path| {
            let bytes = corpus_bytes(name, path);
            let hash: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            json!({"uri": uri(name, path), "digest": format!("sha256:{hash}"), "size": bytes.len()})
        })
        .collect()
}

fn assert_entry(skill: &Skill, name: &str, paths: &[&str]) {
    assert_eq!(skill.uri, uri(name, "SKILL.md"));
    assert_eq!(
        serde_json::to_value(&skill.frontmatter).expect("json"),
        expected_frontmatter(name)
    );
    assert_eq!(skill.frontmatter.name, name);
    assert!(!skill.frontmatter.description.is_empty());
    let SkillResources::Files(files) = &skill.resources else {
        panic!("{name} lists no files");
    };
    let mut got: Vec<Value> = files
        .iter()
        .map(|f| serde_json::to_value(f).expect("json"))
        .collect();
    let mut want = expected_files(name, paths);
    got.sort_by_key(Value::to_string);
    want.sort_by_key(Value::to_string);
    assert_eq!(got, want, "{name}");
}

fn code(error: ServiceError) -> i32 {
    match error {
        ServiceError::McpError(data) => data.code.0,
        other => panic!("expected a JSON-RPC error, got {other:?}"),
    }
}

async fn listed(client: &Client) -> Vec<Skill> {
    let mut skills = client.list_skills(None).await.expect("skills/list").skills;
    skills.sort_by(|a, b| a.name().cmp(b.name()));
    skills
}

async fn read_text(client: &Client, uri: &str) -> String {
    let result = client
        .read_resource(ReadResourceRequestParams::new(uri))
        .await
        .expect("resources/read");
    match result.contents.as_slice() {
        [ResourceContents::TextResourceContents { text, .. }] => text.clone(),
        other => panic!("expected one text content, got {other:?}"),
    }
}

#[tokio::test]
async fn skills_extension_is_declared_with_resources_and_unchanged_tools() {
    for era in [Era::Initialize, Era::Discover] {
        let scratch = Scratch::new();
        let client = connect(&scratch.bundle(), era).await;
        let info = client.peer_info().expect("server info");
        assert!(
            info.capabilities.supports_skills(),
            "{:?}",
            info.capabilities
        );
        assert!(!info.capabilities.supports_directory_read());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.tools.is_some());
        let tools = client.list_all_tools().await.expect("tools/list");
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        assert_eq!(names, ["write", "read", "manage", "check"]);
    }
}

#[tokio::test]
async fn skills_list_returns_the_four_skills_with_frontmatter_and_file_digests() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let skills = listed(&client).await;
    let names: Vec<&str> = skills.iter().map(Skill::name).collect();
    assert_eq!(
        names,
        [
            "living-docs",
            "living-docs-drift",
            "okf-knowledge-format",
            "research-artifacts"
        ]
    );
    for (skill, (name, paths)) in skills.iter().zip(INSTALLABLE) {
        assert_entry(skill, name, paths);
    }
}

#[tokio::test]
async fn skills_get_returns_the_entry_the_list_gave() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    for skill in listed(&client).await {
        let got = client
            .get_skill(GetSkillRequestParams::new(skill.uri.clone()))
            .await
            .expect("skills/get");
        assert_eq!(got.skill, skill);
    }
}

#[tokio::test]
async fn skills_get_rejects_unknown_non_installable_and_malformed_uris() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    for bad in [
        "skill://nope/SKILL.md",
        "skill://living-docs/rules/adr-conventions.md",
        "skill://living-docs/scripts/drift.sh",
        "skill://",
        "skill://../x",
        "skill://%2e%2e/SKILL.md",
        "skill://living-docs/%2e%2e/living-docs/SKILL.md",
        "skill://living-docs/../living-docs/SKILL.md",
        "file:///etc/passwd",
    ] {
        let error = client
            .get_skill(GetSkillRequestParams::new(bad))
            .await
            .expect_err(bad);
        assert_eq!(code(error), -32602, "{bad}");
        assert_eq!(
            listed(&client).await.len(),
            4,
            "still answering after {bad}"
        );
    }
}

#[tokio::test]
async fn skill_resources_list_every_skill_file_and_read_it_byte_for_byte() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let resources = client.list_all_resources().await.expect("resources/list");
    let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
    for (name, paths) in INSTALLABLE {
        for path in paths {
            let want = uri(name, path);
            assert!(
                uris.contains(&want.as_str()),
                "{want} missing from {uris:?}"
            );
            let text = read_text(&client, &want).await;
            assert_eq!(
                text.as_bytes(),
                corpus_bytes(name, path).as_slice(),
                "{want}"
            );
        }
    }
}

#[tokio::test]
async fn skill_resources_reject_uris_outside_the_installable_set() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    for bad in [
        "skill://living-docs/rules/adr-conventions.md",
        "skill://living-docs/../living-docs-drift/SKILL.md",
        "skill://living-docs/%2e%2e/SKILL.md",
        "skill://../SKILL.md",
        "skill://",
        "file:///etc/passwd",
        "https://example.com/SKILL.md",
    ] {
        let result = client
            .read_resource(ReadResourceRequestParams::new(bad))
            .await;
        assert!(result.is_err(), "{bad} was served: {result:?}");
    }
    assert!(client.list_skills(None).await.is_ok(), "still answering");
}
