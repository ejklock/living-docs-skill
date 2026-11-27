//! Resource change events delivered over `subscriptions/listen`, for the
//! server's own writes and for edits made to the bundle from outside.

use crate::common::{call, connect, is_error, message, new_adr, Client, Era, Probe, Scratch, BODY};
use crate::records::{fixture, write_record};
use living_docs_mcp::BundleServer;
use rmcp::model::{GetMeta, ProtocolVersion, RequestId, ServerNotification, SubscriptionFilter};
use rmcp::service::Subscription;
use rmcp::{ClientLifecycleMode, ClientServiceExt, ServiceExt};
use serde_json::json;
use std::fs;
use std::time::Duration;

const ADR_ONE: &str = "record://adr/0001";
const ADR_TWO: &str = "record://adr/0002";
const ADR_ONE_FILE: &str = "adr/0001-use-postgres.md";
const FIRST: Duration = Duration::from_secs(2);
const QUIET: Duration = Duration::from_millis(700);
/// The SDK waits up to five seconds for an open listen to finish after the
/// client closes, then ends the session.
const SDK_DRAIN: Duration = Duration::from_secs(8);

#[derive(Debug, PartialEq)]
enum Event {
    ListChanged,
    Updated(String),
}

fn event_of(notification: ServerNotification, id: &RequestId) -> Event {
    assert_eq!(
        notification.get_meta().subscription_id().as_ref(),
        Some(id),
        "every notification carries the listen request id"
    );
    match notification {
        ServerNotification::ResourceListChangedNotification(_) => Event::ListChanged,
        ServerNotification::ResourceUpdatedNotification(update) => {
            Event::Updated(update.params.uri)
        }
        other => panic!("a notification nobody asked for: {other:?}"),
    }
}

async fn next_event(subscription: &mut Subscription, wait: Duration) -> Option<Event> {
    let id = subscription.id().clone();
    match tokio::time::timeout(wait, subscription.next()).await {
        Ok(Ok(Some(notification))) => Some(event_of(notification, &id)),
        _ => None,
    }
}

/// Every event that arrives until `QUIET` passes with none.
async fn settled(subscription: &mut Subscription) -> Vec<Event> {
    let mut events = Vec::new();
    while let Some(event) = next_event(subscription, QUIET).await {
        events.push(event);
    }
    events
}

async fn listen(client: &Client, uris: &[&str], list_changed: bool) -> Subscription {
    let mut builder = SubscriptionFilter::builder().resource_subscriptions(uris.iter().copied());
    if list_changed {
        builder = builder.resources_list_changed();
    }
    client.listen(builder.build()).await.expect("listen")
}

fn updated(uri: &str) -> Event {
    Event::Updated(uri.to_string())
}

async fn set_description(client: &Client, reference: &str, value: &str) {
    let arguments = json!({
        "operation": "set", "reference": reference, "key": "description", "value": value
    });
    let result = call(client, "write", arguments).await;
    assert!(!is_error(&result), "{}", message(&result));
}

#[tokio::test]
async fn resource_events_acknowledge_only_what_the_server_offers() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;

    let both = listen(&client, &[ADR_ONE], true).await;
    let acknowledged = both.acknowledged().clone();
    assert_eq!(acknowledged.resources_list_changed, Some(true));
    assert_eq!(
        acknowledged.resource_subscriptions,
        Some(vec![ADR_ONE.to_string()])
    );

    let filter = SubscriptionFilter::builder().tools_list_changed().build();
    let mut tools_only = client.listen(filter).await.expect("listen");
    assert_eq!(tools_only.acknowledged(), &SubscriptionFilter::new());
    new_adr(&client, "Use redis", BODY).await;
    assert_eq!(settled(&mut tools_only).await, vec![]);
}

#[tokio::test]
async fn resource_events_server_writes_notify_each_listener_once_for_what_it_asked() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut full = listen(&client, &[ADR_ONE], true).await;
    let mut lists_only = listen(&client, &[], true).await;

    new_adr(&client, "Use redis", BODY).await;
    assert_eq!(settled(&mut full).await, vec![Event::ListChanged]);
    assert_eq!(settled(&mut lists_only).await, vec![Event::ListChanged]);

    set_description(&client, "adr/0001", "Postgres, reworded.").await;
    assert_eq!(settled(&mut full).await, vec![updated(ADR_ONE)]);
    assert_eq!(settled(&mut lists_only).await, vec![]);

    set_description(&client, "adr/0002", "Sqlite, reworded.").await;
    assert_eq!(settled(&mut full).await, vec![]);
    assert_eq!(settled(&mut lists_only).await, vec![]);
}

/// The one event a change must cause: it arrives in time and nothing follows.
async fn expect_once(listener: &mut Subscription, event: Event) {
    assert_eq!(next_event(listener, FIRST).await, Some(event));
    assert_eq!(settled(listener).await, vec![]);
}

async fn outside_listener(scratch: &Scratch) -> (Client, Subscription) {
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let listener = listen(&client, &[ADR_ONE], true).await;
    (client, listener)
}

#[tokio::test]
async fn resource_events_outside_edits_notify_once_for_a_rewritten_record() {
    let scratch = fixture();
    let (_client, mut listener) = outside_listener(&scratch).await;

    let edited = scratch.read(ADR_ONE_FILE) + "\nEdited.\n";
    fs::write(scratch.bundle().join(ADR_ONE_FILE), edited).expect("edit");

    expect_once(&mut listener, updated(ADR_ONE)).await;
}

#[tokio::test]
async fn resource_events_outside_edits_coalesce_a_burst_of_writes() {
    let scratch = fixture();
    let (_client, mut listener) = outside_listener(&scratch).await;
    let file = scratch.bundle().join(ADR_ONE_FILE);

    for round in 0..10 {
        let text = scratch.read(ADR_ONE_FILE) + &format!("\nRound {round}.\n");
        fs::write(&file, text).expect("burst write");
        tokio::time::sleep(Duration::from_millis(8)).await;
    }

    expect_once(&mut listener, updated(ADR_ONE)).await;
}

#[tokio::test]
async fn resource_events_outside_edits_notify_once_for_a_created_record() {
    let scratch = fixture();
    let (_client, mut listener) = outside_listener(&scratch).await;

    write_record(&scratch, "adr/0009-new.md", "ADR", "Brand new", "Accepted");

    expect_once(&mut listener, Event::ListChanged).await;
}

#[tokio::test]
async fn resource_events_outside_edits_notify_once_for_a_deleted_record() {
    let scratch = fixture();
    let (_client, mut listener) = outside_listener(&scratch).await;

    fs::remove_file(scratch.bundle().join("adr/0002-use-sqlite.md")).expect("delete");

    expect_once(&mut listener, Event::ListChanged).await;
}

#[tokio::test]
async fn resource_events_outside_edits_ignore_files_that_are_not_records() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut listener = listen(&client, &[ADR_TWO], true).await;

    fs::write(scratch.bundle().join("adr/index.md"), "# ADRs\n").expect("index");
    fs::write(scratch.outside().join("note.md"), "outside the bundle").expect("outside");
    fs::write(scratch.bundle().join("notes.txt"), "not markdown").expect("text file");

    assert_eq!(settled(&mut listener).await, vec![]);
}

#[tokio::test]
async fn resource_events_lifecycle_stop_at_cancel() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut cancelled = listen(&client, &[ADR_ONE], true).await;
    let mut control = listen(&client, &[ADR_ONE], true).await;

    cancelled.cancel().await.expect("cancel");
    let file = scratch.bundle().join(ADR_ONE_FILE);
    fs::write(&file, scratch.read(ADR_ONE_FILE) + "\nEdited.\n").expect("edit");

    assert_eq!(
        next_event(&mut control, FIRST).await,
        Some(updated(ADR_ONE))
    );
    assert_eq!(settled(&mut cancelled).await, vec![]);
    let tools = client.list_all_tools().await.expect("still answering");
    assert_eq!(tools.len(), 4);
}

#[tokio::test]
async fn resource_events_lifecycle_follow_a_bundle_created_after_the_listen() {
    let scratch = Scratch::new();
    let missing = scratch.outside().join("nowhere");
    let client = connect(&missing, Era::Discover).await;
    let mut listener = listen(&client, &[], true).await;
    assert_eq!(listener.acknowledged().resources_list_changed, Some(true));

    new_adr(&client, "Use postgres", BODY).await;

    assert_eq!(
        next_event(&mut listener, FIRST).await,
        Some(Event::ListChanged)
    );
    assert_eq!(settled(&mut listener).await, vec![]);
    assert!(client.list_all_tools().await.is_ok());
}

#[tokio::test]
async fn resource_events_lifecycle_end_the_server_when_the_client_closes() {
    let scratch = fixture();
    let (server_io, client_io) = tokio::io::duplex(1 << 16);
    let server = BundleServer::new(&scratch.bundle());
    let serving = tokio::spawn(async move {
        let running = server.serve(server_io).await.expect("serve");
        running.waiting().await
    });
    let client = Probe
        .serve_with_lifecycle(
            client_io,
            ClientLifecycleMode::Discover {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
            },
        )
        .await
        .expect("server/discover startup");
    let _listener = listen(&client, &[ADR_ONE], true).await;

    client.cancel().await.expect("close");

    let ended = tokio::time::timeout(SDK_DRAIN, serving).await;
    assert!(ended.expect("the server ends").expect("no panic").is_ok());
}
