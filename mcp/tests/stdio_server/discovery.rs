use crate::common::{connect, Era, Scratch};
use rmcp::model::ProtocolVersion;
use serde_json::Value;

fn operations(schema: &Value) -> Vec<String> {
    schema["oneOf"]
        .as_array()
        .map(|variants| {
            variants
                .iter()
                .filter_map(|variant| variant.pointer("/properties/operation/const"))
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn lists_the_four_tools_in_a_fixed_order_with_their_operations() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    for _ in 0..2 {
        let tools = client.list_all_tools().await.expect("tools/list");
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_ref()).collect();
        assert_eq!(names, ["write", "read", "manage", "check"]);
        let schema = |name: &str| {
            let tool = tools.iter().find(|tool| tool.name == name).expect("tool");
            Value::Object((*tool.input_schema).clone())
        };
        assert_eq!(operations(&schema("write")), ["new", "set", "supersede"]);
        assert_eq!(operations(&schema("read")), ["records"]);
        assert_eq!(operations(&schema("manage")), ["index", "fmt"]);
        assert!(schema("check").is_object());
    }
}

#[tokio::test]
async fn protocol_eras_discover_carries_the_teaching_instructions() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let info = client.peer_info().expect("discover result retained");
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    let instructions = info
        .instructions
        .clone()
        .expect("instructions")
        .to_lowercase();
    for tool in ["write", "read", "manage", "check"] {
        assert!(instructions.contains(tool), "no {tool} in: {instructions}");
    }
    assert!(instructions.contains("before deciding"), "{instructions}");
    assert!(instructions.contains(&scratch.bundle().display().to_string().to_lowercase()));
}

#[tokio::test]
async fn protocol_eras_initialize_handshake_still_lists_tools() {
    let scratch = Scratch::new();
    let client = connect(&scratch.bundle(), Era::Initialize).await;
    let info = client.peer_info().expect("initialize result retained");
    assert_eq!(info.protocol_version, ProtocolVersion::V_2025_11_25);
    assert_eq!(client.list_all_tools().await.expect("tools/list").len(), 4);
}
