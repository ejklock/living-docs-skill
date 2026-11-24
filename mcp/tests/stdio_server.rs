#[path = "stdio_server/authoring.rs"]
mod authoring;
#[path = "stdio_server/boundary.rs"]
mod boundary;
#[path = "stdio_server/check_and_read.rs"]
mod check_and_read;
#[path = "stdio_server/common.rs"]
mod common;
#[path = "stdio_server/discovery.rs"]
mod discovery;
#[path = "stdio_server/guide.rs"]
mod guide;

/// The `manage` operation `export_view`: the TLA+ module of a flat state view
/// as the tool's text, and the refusals of `manage view --export tla`.
mod export_view {
    use crate::common::{call, connect, is_error, message, Era, Scratch};
    use rmcp::model::CallToolResult;
    use serde_json::json;
    use std::fs;

    const FLAT: &str = "stateDiagram-v2\n[*] --> pending\npending --> paid: pay\npaid --> shipped\nshipped --> [*]\npending --> cancelled: customer cancels\ncancelled --> [*]\n";
    const VIEW: &str = "architecture/order-lifecycle";
    const MODULE: &str = r#"---- MODULE order_lifecycle ----
VARIABLE state

States == {"pending", "paid", "shipped", "cancelled"}

Terminal == {"shipped", "cancelled"}

Init == state = "pending"

\* pending --> paid: pay
T1 == state = "pending" /\ state' = "paid"

\* paid --> shipped
T2 == state = "paid" /\ state' = "shipped"

\* pending --> cancelled: customer cancels
T3 == state = "pending" /\ state' = "cancelled"

Next == T1 \/ T2 \/ T3 \/ (state \in Terminal /\ state' = state)

TypeOK == state \in States

Spec == Init /\ [][Next]_state /\ WF_state(Next)
====
"#;

    fn bundle_with(kind: &str, fence: &str) -> Scratch {
        let scratch = Scratch::new();
        let dir = scratch.bundle().join("architecture");
        fs::create_dir_all(&dir).expect("architecture dir");
        let record = format!(
            "---\ntype: Architecture View\ntitle: Order Lifecycle\ndescription: The lifecycle.\nkind: {kind}\ntimestamp: 2026-11-20T10:00:00Z\n---\n\n# Order Lifecycle\n\n```mermaid\n{fence}```\n\n## Drift instrument\n\nDrift caught by: the check.\n"
        );
        fs::write(dir.join("order-lifecycle.md"), record).expect("view record");
        scratch
    }

    async fn export(scratch: &Scratch, view: &str) -> CallToolResult {
        let client = connect(&scratch.bundle(), Era::Initialize).await;
        let args = json!({"operation": "export_view", "view": view, "format": "tla"});
        call(&client, "manage", args).await
    }

    async fn refusal(kind: &str, fence: &str, view: &str) -> String {
        let scratch = bundle_with(kind, fence);
        let result = export(&scratch, view).await;
        assert!(is_error(&result), "{result:?}");
        assert!(result.structured_content.is_none(), "{result:?}");
        message(&result)
    }

    #[tokio::test]
    async fn a_flat_state_view_returns_the_exact_module_as_text() {
        let scratch = bundle_with("state", FLAT);
        let result = export(&scratch, VIEW).await;
        assert!(!is_error(&result), "{}", message(&result));
        assert_eq!(message(&result), MODULE);
    }

    #[tokio::test]
    async fn a_composite_state_is_a_tool_error_naming_it() {
        let fence = "stateDiagram-v2\n[*] --> Outer\nstate Outer {\n[*] --> Inner\nInner --> [*]\n}\nOuter --> [*]\n";
        let text = refusal("state", fence, VIEW).await;
        assert!(text.contains("composite state 'Outer'"), "{text}");
    }

    #[tokio::test]
    async fn concurrent_regions_are_a_tool_error() {
        let fence = "stateDiagram-v2\n[*] --> A\nstate A {\nB --> C\n--\nD --> E\n}\n";
        let text = refusal("state", fence, VIEW).await;
        assert!(text.contains("concurrent regions"), "{text}");
    }

    #[tokio::test]
    async fn a_view_that_is_not_a_state_view_is_a_tool_error() {
        let text = refusal("flow", FLAT, VIEW).await;
        assert!(text.contains("not a state view"), "{text}");
    }

    #[tokio::test]
    async fn an_unknown_view_is_a_tool_error() {
        let text = refusal("state", FLAT, "architecture/missing").await;
        assert!(text.contains("no record found"), "{text}");
    }

    #[tokio::test]
    async fn a_format_other_than_tla_is_not_an_export() {
        let scratch = bundle_with("state", FLAT);
        let client = connect(&scratch.bundle(), Era::Initialize).await;
        let args = json!({"operation": "export_view", "view": VIEW, "format": "alloy"});
        let arguments = args.as_object().cloned().expect("object");
        let params = rmcp::model::CallToolRequestParams::new("manage").with_arguments(arguments);
        let outcome = client.call_tool(params).await;
        let text = match outcome {
            Ok(result) => {
                assert!(is_error(&result), "{result:?}");
                message(&result)
            }
            Err(error) => error.to_string(),
        };
        assert!(text.contains("alloy"), "{text}");
    }
}
