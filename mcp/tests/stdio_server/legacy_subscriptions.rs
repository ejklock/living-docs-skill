//! Resource subscriptions for a client on the `initialize` protocol revision:
//! `resources/subscribe` and the notifications it brings.

use crate::common::BODY;
use crate::records::fixture;
use living_docs_mcp::BundleServer;
use rmcp::model::{
    CallToolRequestParams, ResourceUpdatedNotificationParam, SubscribeRequestParams,
    UnsubscribeRequestParams,
};
use rmcp::service::NotificationContext;
use rmcp::{ClientHandler, RoleClient, ServiceExt};
use serde_json::{json, Value};
use std::fs;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

const ADR_ONE: &str = "record://adr/0001";
const ADR_ONE_FILE: &str = "adr/0001-use-postgres.md";
const FIRST: Duration = Duration::from_secs(2);
const QUIET: Duration = Duration::from_millis(700);

#[derive(Debug, PartialEq)]
enum Event {
    ListChanged,
    Updated(String),
}

/// A client that reports every resource notification it receives.
#[derive(Clone)]
struct Collector(UnboundedSender<Event>);

impl ClientHandler for Collector {
    async fn on_resource_updated(
        &self,
        params: ResourceUpdatedNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        let _ = self.0.send(Event::Updated(params.uri));
    }

    async fn on_resource_list_changed(&self, _context: NotificationContext<RoleClient>) {
        let _ = self.0.send(Event::ListChanged);
    }
}

type Client = rmcp::service::RunningService<RoleClient, Collector>;

async fn connect(bundle: &std::path::Path) -> (Client, UnboundedReceiver<Event>) {
    let (server_io, client_io) = tokio::io::duplex(1 << 16);
    let server = BundleServer::new(bundle);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    let (sender, events) = unbounded_channel();
    let client = Collector(sender)
        .serve(client_io)
        .await
        .expect("initialize handshake");
    (client, events)
}

#[allow(deprecated)]
async fn subscribe(client: &Client, uri: &str) {
    client
        .subscribe(SubscribeRequestParams::new(uri))
        .await
        .expect("the server accepts the subscription");
}

#[allow(deprecated)]
async fn unsubscribe(client: &Client, uri: &str) {
    client
        .unsubscribe(UnsubscribeRequestParams::new(uri))
        .await
        .expect("the server accepts the unsubscription");
}

async fn write(client: &Client, arguments: Value) {
    let Value::Object(arguments) = arguments else {
        panic!("tool arguments are an object");
    };
    let result = client
        .call_tool(CallToolRequestParams::new("write").with_arguments(arguments))
        .await
        .expect("the server answers the call");
    assert_ne!(result.is_error, Some(true), "{result:?}");
}

async fn set_description(client: &Client, reference: &str, value: &str) {
    let arguments = json!({
        "operation": "set", "reference": reference, "key": "description", "value": value
    });
    write(client, arguments).await;
}

async fn next_event(events: &mut UnboundedReceiver<Event>, wait: Duration) -> Option<Event> {
    tokio::time::timeout(wait, events.recv())
        .await
        .ok()
        .flatten()
}

/// Every event that arrives until `QUIET` passes with none.
async fn settled(events: &mut UnboundedReceiver<Event>) -> Vec<Event> {
    let mut seen = Vec::new();
    while let Some(event) = next_event(events, QUIET).await {
        seen.push(event);
    }
    seen
}

fn updated(uri: &str) -> Event {
    Event::Updated(uri.to_string())
}

/// The one event a change must cause: it arrives in time and nothing follows.
async fn expect_once(events: &mut UnboundedReceiver<Event>, event: Event) {
    assert_eq!(next_event(events, FIRST).await, Some(event));
    assert_eq!(settled(events).await, vec![]);
}

#[tokio::test]
async fn legacy_subscription_reports_a_server_write_to_the_subscribed_record() {
    let scratch = fixture();
    let (client, mut events) = connect(&scratch.bundle()).await;
    subscribe(&client, ADR_ONE).await;

    set_description(&client, "adr/0001", "Postgres, reworded.").await;

    expect_once(&mut events, updated(ADR_ONE)).await;
}

#[tokio::test]
async fn legacy_subscription_reports_an_outside_rewrite_of_the_subscribed_record() {
    let scratch = fixture();
    let (client, mut events) = connect(&scratch.bundle()).await;
    subscribe(&client, ADR_ONE).await;

    let edited = scratch.read(ADR_ONE_FILE) + "\nEdited.\n";
    fs::write(scratch.bundle().join(ADR_ONE_FILE), edited).expect("edit");

    expect_once(&mut events, updated(ADR_ONE)).await;
}

#[tokio::test]
async fn legacy_subscription_reports_a_new_record_as_a_list_change() {
    let scratch = fixture();
    let (client, mut events) = connect(&scratch.bundle()).await;
    subscribe(&client, ADR_ONE).await;

    let arguments = json!({
        "operation": "new", "type": "adr", "title": "Use redis",
        "description": "A decision.", "owner": "Eve", "body": BODY
    });
    write(&client, arguments).await;

    expect_once(&mut events, Event::ListChanged).await;
}

#[tokio::test]
async fn legacy_subscription_ignores_a_change_to_another_record() {
    let scratch = fixture();
    let (client, mut events) = connect(&scratch.bundle()).await;
    subscribe(&client, ADR_ONE).await;

    set_description(&client, "adr/0002", "Sqlite, reworded.").await;

    assert_eq!(settled(&mut events).await, vec![]);
}

#[tokio::test]
async fn legacy_subscription_stops_reporting_after_unsubscribe() {
    let scratch = fixture();
    let (client, mut events) = connect(&scratch.bundle()).await;
    subscribe(&client, ADR_ONE).await;
    unsubscribe(&client, ADR_ONE).await;

    set_description(&client, "adr/0001", "Postgres, reworded.").await;
    let edited = scratch.read(ADR_ONE_FILE) + "\nEdited.\n";
    fs::write(scratch.bundle().join(ADR_ONE_FILE), edited).expect("edit");

    assert_eq!(settled(&mut events).await, vec![]);
}

#[tokio::test]
async fn legacy_subscription_never_notifies_a_session_that_did_not_subscribe() {
    let scratch = fixture();
    let (client, mut events) = connect(&scratch.bundle()).await;

    set_description(&client, "adr/0001", "Postgres, reworded.").await;
    let edited = scratch.read(ADR_ONE_FILE) + "\nEdited.\n";
    fs::write(scratch.bundle().join(ADR_ONE_FILE), edited).expect("edit");

    assert_eq!(settled(&mut events).await, vec![]);
}
