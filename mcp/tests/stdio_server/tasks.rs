use crate::common::{call, connect, data, is_error, message, new_adr, Era, Scratch, BODY};
use living_docs_mcp::BundleServer;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, CancelTaskParams, ClientCapabilities,
    ClientConfig, ErrorCode, GetTaskParams, Implementation, ProtocolVersion, Task, TaskPayload,
};
use rmcp::{ClientLifecycleMode, ClientServiceExt, RoleClient, ServiceError, ServiceExt};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::time::Duration;

type Client = rmcp::service::RunningService<RoleClient, ClientConfig>;

const POLLS: usize = 600;

async fn connect_as(
    bundle: &Path,
    threshold: usize,
    declares_tasks: bool,
    lifecycle: ClientLifecycleMode,
) -> Client {
    let (server_io, client_io) = tokio::io::duplex(1 << 16);
    let server = BundleServer::with_check_task_threshold(bundle, threshold);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let mut capabilities = ClientCapabilities::builder();
    if declares_tasks {
        capabilities = capabilities.enable_tasks();
    }
    ClientConfig::new(capabilities.build(), Implementation::from_build_env())
        .serve_with_lifecycle(client_io, lifecycle)
        .await
        .expect("client startup")
}

async fn connect_with_threshold(bundle: &Path, threshold: usize, declares_tasks: bool) -> Client {
    let lifecycle = ClientLifecycleMode::Initialize;
    connect_as(bundle, threshold, declares_tasks, lifecycle).await
}

async fn connect_discover_with_threshold(bundle: &Path, declares_tasks: bool) -> Client {
    let preferred_versions = vec![ProtocolVersion::V_2026_07_28];
    let lifecycle = ClientLifecycleMode::Discover { preferred_versions };
    connect_as(bundle, 1, declares_tasks, lifecycle).await
}

/// Writes one record with no owner, so `require_owner` changes the report.
async fn unowned_adr(bundle: &Path) {
    let client = connect_with_threshold(bundle, usize::MAX, false).await;
    let args = json!({"operation": "new", "type": "adr", "title": "Decision", "body": BODY,
        "description": "A decision."});
    let result = call_inline(&client, "write", args).await;
    assert!(!is_error(&result), "{}", message(&result));
}

async fn adrs(client: &Client, count: usize) {
    for n in 1..=count {
        let args = json!({
            "operation": "new", "type": "adr", "title": format!("Decision {n}"),
            "description": "A decision.", "owner": "Eve", "body": BODY
        });
        let result = check_once(client, "write", args).await;
        assert!(matches!(result, CallToolResponse::Complete(ref r) if !is_error(r)));
    }
}

async fn check_once(client: &Client, tool: &'static str, arguments: Value) -> CallToolResponse {
    let Value::Object(arguments) = arguments else {
        panic!("tool arguments are an object");
    };
    client
        .call_tool_once(CallToolRequestParams::new(tool).with_arguments(arguments))
        .await
        .expect("the server answers the call")
}

fn expect_created(response: CallToolResponse) -> Task {
    match response {
        CallToolResponse::Task(created) => created.task.clone(),
        other => panic!("expected a task, got {other:?}"),
    }
}

fn expect_task(response: CallToolResponse) -> String {
    expect_created(response).task_id
}

fn expect_inline(response: CallToolResponse) -> CallToolResult {
    match response {
        CallToolResponse::Complete(result) => result,
        other => panic!("expected an inline answer, got {other:?}"),
    }
}

async fn settled(client: &Client, task_id: &str) -> TaskPayload {
    for _ in 0..POLLS {
        let info = client
            .peer()
            .get_task(GetTaskParams::new(task_id))
            .await
            .expect("tasks/get answers");
        if info.task.status().is_terminal() {
            return info.task.payload;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the task did not settle");
}

fn report_of(payload: TaskPayload) -> Value {
    match payload {
        TaskPayload::Completed { result } => {
            let result: CallToolResult =
                serde_json::from_value(Value::Object(result)).expect("a tool result");
            data(&result)
        }
        other => panic!("expected a completed task, got {other:?}"),
    }
}

async fn inline_report(bundle: &Path, args: Value) -> Value {
    let client = connect_with_threshold(bundle, 1, false).await;
    data(&expect_inline(check_once(&client, "check", args).await))
}

#[tokio::test]
async fn check_task_declared_in_both_protocol_eras() {
    for era in [Era::Initialize, Era::Discover] {
        let scratch = Scratch::new();
        let client = connect(&scratch.bundle(), era).await;
        let capabilities = &client.peer_info().expect("server info").capabilities;
        assert!(capabilities.supports_tasks(), "{capabilities:?}");
        assert!(capabilities.tools.is_some() && capabilities.resources.is_some());
    }
}

#[tokio::test]
async fn check_task_threshold_returns_a_task_at_the_count_with_the_inline_report() {
    let scratch = Scratch::new();
    let client = connect_with_threshold(&scratch.bundle(), 3, true).await;
    adrs(&client, 3).await;
    let task_id = expect_task(check_once(&client, "check", json!({})).await);
    let report = report_of(settled(&client, &task_id).await);
    assert_eq!(report, inline_report(&scratch.bundle(), json!({})).await);
    assert_eq!(report["ok"], true);
}

#[tokio::test]
async fn check_task_threshold_runs_the_check_with_the_call_arguments() {
    let scratch = Scratch::new();
    unowned_adr(&scratch.bundle()).await;
    let client = connect_with_threshold(&scratch.bundle(), 1, true).await;
    let args = json!({"require_owner": true});
    let created = expect_created(check_once(&client, "check", args.clone()).await);
    assert_eq!(created.poll_interval_ms, Some(2000));
    let report = report_of(settled(&client, &created.task_id).await);
    assert_eq!(report, inline_report(&scratch.bundle(), args).await);
    assert_eq!(report["ok"], false);
    let task_id = expect_task(check_once(&client, "check", json!({})).await);
    assert_eq!(report_of(settled(&client, &task_id).await)["ok"], true);
}

#[tokio::test]
async fn check_task_threshold_serves_the_2026_07_28_revision_that_declares_tasks_per_request() {
    let scratch = Scratch::new();
    unowned_adr(&scratch.bundle()).await;
    let declared = connect_discover_with_threshold(&scratch.bundle(), true).await;
    let task_id = expect_task(check_once(&declared, "check", json!({})).await);
    assert_eq!(report_of(settled(&declared, &task_id).await)["ok"], true);
    let silent = connect_discover_with_threshold(&scratch.bundle(), false).await;
    let result = expect_inline(check_once(&silent, "check", json!({})).await);
    assert_eq!(data(&result)["ok"], true);
}

#[tokio::test]
async fn check_task_threshold_answers_inline_one_record_below() {
    let scratch = Scratch::new();
    let client = connect_with_threshold(&scratch.bundle(), 3, true).await;
    adrs(&client, 2).await;
    let result = expect_inline(check_once(&client, "check", json!({})).await);
    assert_eq!(data(&result)["ok"], true);
}

#[tokio::test]
async fn check_task_threshold_returns_a_task_one_record_above() {
    let scratch = Scratch::new();
    let client = connect_with_threshold(&scratch.bundle(), 3, true).await;
    adrs(&client, 4).await;
    expect_task(check_once(&client, "check", json!({})).await);
}

#[tokio::test]
async fn check_task_threshold_ignores_a_retired_record() {
    let scratch = Scratch::new();
    let client = connect_with_threshold(&scratch.bundle(), 3, true).await;
    adrs(&client, 3).await;
    let args = json!({"operation": "supersede", "old": "adr/0001", "new": "adr/0002"});
    let superseded = call_inline(&client, "write", args).await;
    assert!(!is_error(&superseded), "{}", message(&superseded));
    expect_inline(check_once(&client, "check", json!({})).await);
}

async fn call_inline(client: &Client, tool: &'static str, arguments: Value) -> CallToolResult {
    expect_inline(check_once(client, tool, arguments).await)
}

#[tokio::test]
async fn check_task_threshold_answers_a_missing_bundle_inline_with_the_error() {
    let scratch = Scratch::new();
    let absent = scratch.bundle().join("absent");
    let client = connect_with_threshold(&absent, 1, true).await;
    let result = expect_inline(check_once(&client, "check", json!({})).await);
    assert!(is_error(&result));
    assert!(message(&result).contains("bundle root not found"));
}

#[tokio::test]
async fn check_task_without_extension_answers_inline_at_and_above_the_count() {
    let scratch = Scratch::new();
    let client = connect_with_threshold(&scratch.bundle(), 3, false).await;
    adrs(&client, 4).await;
    let result = expect_inline(check_once(&client, "check", json!({})).await);
    assert_eq!(data(&result)["ok"], true);
}

#[tokio::test]
async fn check_task_without_extension_never_applies_to_the_other_tools() {
    for declares_tasks in [false, true] {
        let scratch = Scratch::new();
        let client = connect_with_threshold(&scratch.bundle(), 1, declares_tasks).await;
        adrs(&client, 3).await;
        let calls = [
            ("read", json!({"operation": "records"})),
            ("manage", json!({"operation": "index"})),
            (
                "write",
                json!({"operation": "set", "reference": "adr/0001", "key": "title", "value": "Renamed"}),
            ),
        ];
        for (tool, arguments) in calls {
            let result = call_inline(&client, tool, arguments).await;
            assert!(!is_error(&result), "{tool}: {}", message(&result));
        }
    }
}

fn seed_records(bundle: &Path, count: usize) {
    fs::create_dir_all(bundle.join("adr")).expect("adr directory");
    for n in 1..=count {
        let text = format!(
            "---\ntype: ADR\ntitle: Decision {n}\ndescription: A decision.\nstatus: Accepted\n---\n\n# {n:04}. Decision {n}\n\n{BODY}"
        );
        fs::write(bundle.join(format!("adr/{n:04}-decision-{n}.md")), text).expect("record");
    }
}

#[tokio::test]
async fn check_task_cancel_ends_the_task_cancelled_without_a_report() {
    let scratch = Scratch::new();
    seed_records(&scratch.bundle(), 600);
    let client = connect_with_threshold(&scratch.bundle(), 1, true).await;
    let task_id = expect_task(check_once(&client, "check", json!({})).await);
    client
        .peer()
        .cancel_task(CancelTaskParams::new(task_id.clone()))
        .await
        .expect("tasks/cancel is acknowledged");
    assert!(matches!(
        settled(&client, &task_id).await,
        TaskPayload::Cancelled
    ));
}

#[tokio::test]
async fn check_task_cancel_of_an_unknown_task_is_an_error_and_the_server_keeps_answering() {
    let scratch = Scratch::new();
    let client = connect_with_threshold(&scratch.bundle(), 1, true).await;
    let peer = client.peer();
    for failure in [
        peer.get_task(GetTaskParams::new("no-such-task"))
            .await
            .map(|_| ()),
        peer.cancel_task(CancelTaskParams::new("no-such-task"))
            .await,
    ] {
        match failure {
            Err(ServiceError::McpError(error)) => assert_eq!(error.code, ErrorCode::INVALID_PARAMS),
            other => panic!("expected invalid params, got {other:?}"),
        }
    }
    let records = call_inline(&client, "read", json!({"operation": "records"})).await;
    assert!(!is_error(&records));
    let after = connect(&scratch.bundle(), Era::Initialize).await;
    new_adr(&after, "After the failures", BODY).await;
    assert!(!is_error(&call(&after, "check", json!({})).await));
}
