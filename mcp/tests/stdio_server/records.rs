//! Records in force served as `record://` resources beside the skill files.

use crate::common::{call, connect, data, is_error, message, Client, Era, Scratch};
use rmcp::model::{ReadResourceRequestParams, ResourceContents};
use serde_json::json;
use std::fs;
use std::path::Path;

/// Writes one record file under the bundle with the given frontmatter fields.
pub fn write_record(scratch: &Scratch, relative: &str, kind: &str, title: &str, status: &str) {
    let path = scratch.bundle().join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("record dir");
    let status = if status.is_empty() {
        String::new()
    } else {
        format!("status: {status}\n")
    };
    let text = format!(
        "---\ntype: {kind}\ntitle: {title}\ndescription: About {title}.\n{status}---\n\n# {title}\n\nBody of {title}.\n"
    );
    fs::write(path, text).expect("record file");
}

/// Every file of the fixture bundle: path, doc type, title, status.
const FILES: [(&str, &str, &str, &str); 8] = [
    (
        "adr/0001-use-postgres.md",
        "ADR",
        "Use postgres",
        "Accepted",
    ),
    ("adr/0002-use-sqlite.md", "ADR", "Use sqlite", "Accepted"),
    ("adr/0003-old.md", "ADR", "Old way", "Superseded"),
    ("issues/0001-open-bug.md", "Issue", "Open bug", "open"),
    ("issues/0002-done-bug.md", "Issue", "Done bug", "closed"),
    ("architecture/crates.md", "Architecture View", "Crates", ""),
    (
        "constitution.md",
        "Constitution",
        "The constitution",
        "Draft",
    ),
    ("context/billing/invoice.md", "Context", "Invoice", ""),
];

/// A bundle holding every kind of record, in force and retired.
pub fn fixture() -> Scratch {
    let scratch = Scratch::new();
    for (relative, kind, title, status) in FILES {
        write_record(&scratch, relative, kind, title, status);
    }
    scratch
}

const RECORDS: [(&str, &str, &str); 6] = [
    (
        "record://adr/0001",
        "Use postgres",
        "adr/0001-use-postgres.md",
    ),
    ("record://adr/0002", "Use sqlite", "adr/0002-use-sqlite.md"),
    ("record://issue/0001", "Open bug", "issues/0001-open-bug.md"),
    ("record://view/crates", "Crates", "architecture/crates.md"),
    (
        "record://constitution",
        "The constitution",
        "constitution.md",
    ),
    (
        "record://context/billing/invoice.md",
        "Invoice",
        "context/billing/invoice.md",
    ),
];

async fn listed(client: &Client) -> Vec<rmcp::model::Resource> {
    client.list_all_resources().await.expect("resources/list")
}

async fn read_text(client: &Client, uri: &str) -> Result<String, String> {
    let result = client
        .read_resource(ReadResourceRequestParams::new(uri))
        .await
        .map_err(|error| error.to_string())?;
    match result.contents.first() {
        Some(ResourceContents::TextResourceContents { text, .. }) => Ok(text.clone()),
        other => Err(format!("not text: {other:?}")),
    }
}

#[tokio::test]
async fn record_resources_list_every_record_in_force_with_its_uri_title_and_description() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Initialize).await;

    let resources = listed(&client).await;

    let records: Vec<_> = resources
        .iter()
        .filter(|resource| resource.uri.starts_with("record://"))
        .collect();
    let mut found: Vec<&str> = records.iter().map(|r| r.uri.as_str()).collect();
    let mut expected: Vec<&str> = RECORDS.iter().map(|(uri, ..)| *uri).collect();
    found.sort_unstable();
    expected.sort_unstable();
    assert_eq!(found, expected, "superseded ADR and closed issue stay out");
    for (uri, title, _) in RECORDS {
        let resource = records.iter().find(|r| r.uri == uri).expect(uri);
        assert_eq!(resource.name, title);
        assert_eq!(
            resource.description.as_deref(),
            Some(format!("About {title}.").as_str())
        );
        assert_eq!(
            resource.mime_type.as_deref(),
            Some("text/markdown"),
            "{uri}"
        );
    }
    let skills = resources
        .iter()
        .filter(|resource| resource.uri.starts_with("skill://"))
        .count();
    assert_eq!(skills, 8, "every skill file is still listed");
}

#[tokio::test]
async fn record_resources_list_only_skill_files_on_a_missing_bundle() {
    let scratch = Scratch::new();
    let missing = scratch.outside().join("nowhere");
    let client = connect(&missing, Era::Initialize).await;

    let resources = listed(&client).await;

    assert_eq!(resources.len(), 8);
    assert!(resources.iter().all(|r| r.uri.starts_with("skill://")));
}

#[tokio::test]
async fn record_resources_read_returns_each_file_exactly() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Initialize).await;

    for (uri, _, relative) in RECORDS {
        let text = read_text(&client, uri).await.expect(uri);
        assert_eq!(text, scratch.read(relative), "{uri}");
    }
    let skill = read_text(&client, "skill://living-docs/SKILL.md").await;
    assert!(skill.expect("skill still reads").contains("living-docs"));
}

#[cfg(unix)]
fn link_outside(scratch: &Scratch) {
    let secret = scratch.outside().join("secret.md");
    fs::write(
        &secret,
        "---\ntype: Context\ntitle: Secret\ndescription: Outside.\n---\n\nHidden.\n",
    )
    .expect("outside file");
    let link = scratch.bundle().join("context/link.md");
    std::os::unix::fs::symlink(&secret, link).expect("symlink");
}

#[cfg(unix)]
#[tokio::test]
async fn record_resources_read_refuses_what_is_not_a_record_in_the_bundle() {
    let scratch = fixture();
    fs::write(scratch.bundle().join("../x.md"), "outside the bundle").expect("x.md");
    link_outside(&scratch);
    let client = connect(&scratch.bundle(), Era::Initialize).await;

    let refused = [
        "record://adr/0099",
        "record://adr/0003",
        "record://adr/../../etc/passwd",
        "record://context/../../x.md",
        "record://context/%2e%2e/%2e%2e/x.md",
        "record://adr/%2e%2e/%2e%2e/etc/passwd",
        "record://context/link.md",
        "record://",
    ];
    for uri in refused {
        let outcome = read_text(&client, uri).await;
        assert!(outcome.is_err(), "{uri} must not read: {outcome:?}");
    }
    let uris: Vec<String> = listed(&client).await.into_iter().map(|r| r.uri).collect();
    assert!(!uris.iter().any(|uri| uri.contains("link.md")), "{uris:?}");
    assert!(Path::new(&scratch.bundle().join("../x.md")).exists());
}

fn record_titles(resources: &[rmcp::model::Resource]) -> Vec<String> {
    let mut titles: Vec<String> = resources
        .iter()
        .filter(|resource| resource.uri.starts_with("record://"))
        .map(|resource| resource.name.clone())
        .collect();
    titles.sort_unstable();
    titles
}

#[tokio::test]
async fn record_resources_list_the_same_records_the_read_tool_returns() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Initialize).await;

    let result = call(&client, "read", json!({"operation": "records"})).await;
    assert!(!is_error(&result), "{}", message(&result));
    let mut from_tool: Vec<String> = data(&result)["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|record| record["title"].as_str().expect("title").to_string())
        .collect();
    from_tool.sort_unstable();

    assert_eq!(record_titles(&listed(&client).await), from_tool);
}

#[tokio::test]
async fn record_resources_skip_an_index_file_that_claims_a_record_type() {
    let scratch = fixture();
    write_record(
        &scratch,
        "adr/index.md",
        "ADR",
        "Index of decisions",
        "Accepted",
    );
    let client = connect(&scratch.bundle(), Era::Initialize).await;

    let resources = listed(&client).await;

    let titles = record_titles(&resources);
    assert!(
        !titles.contains(&"Index of decisions".to_string()),
        "{titles:?}"
    );
    assert!(read_text(&client, "record://adr/index.md").await.is_err());
}

const FATURACAO: &str = "context/faturação/nota fiscal.md";
const FATURACAO_URI: &str = "record://context/fatura%C3%A7%C3%A3o/nota%20fiscal.md";

#[tokio::test]
async fn record_resources_encode_the_path_of_a_record_named_with_spaces_or_accents() {
    let scratch = fixture();
    write_record(&scratch, FATURACAO, "Context", "Nota fiscal", "");
    let client = connect(&scratch.bundle(), Era::Initialize).await;

    let uris: Vec<String> = listed(&client).await.into_iter().map(|r| r.uri).collect();
    assert!(
        uris.iter().any(|uri| uri == FATURACAO_URI),
        "the listed uris are percent-encoded: {uris:?}"
    );
    let text = read_text(&client, FATURACAO_URI)
        .await
        .expect("encoded uri");
    assert_eq!(text, scratch.read(FATURACAO));

    let refused = [
        "record://context/faturação/nota fiscal.md",
        "record://context/%2e%2e/context/fatura%C3%A7%C3%A3o/nota%20fiscal.md",
    ];
    for uri in refused {
        assert!(
            read_text(&client, uri).await.is_err(),
            "{uri} must not read"
        );
    }
}
