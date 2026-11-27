//! Burst coalescing of server writes, the reserved-file rule and the watcher's
//! lifecycle, over `subscriptions/listen`.

use super::{
    expect_once, listen, next_event, set_description, settled, updated, Event, ADR_ONE, ADR_TWO,
    FIRST,
};
use crate::common::{connect, new_adr, Era, BODY};
use crate::records::{fixture, write_record};
use std::fs;
use std::time::Duration;

#[tokio::test]
async fn resource_events_server_writes_coalesce_a_burst_into_one_update() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut listener = listen(&client, &[ADR_ONE], true).await;

    for round in 0..3 {
        set_description(&client, "adr/0001", &format!("Reworded {round}.")).await;
    }

    expect_once(&mut listener, updated(ADR_ONE)).await;
}

#[tokio::test]
async fn resource_events_server_writes_report_a_single_write_in_time() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut listener = listen(&client, &[ADR_ONE], true).await;

    set_description(&client, "adr/0001", "Reworded.").await;

    expect_once(&mut listener, updated(ADR_ONE)).await;
}

#[tokio::test]
async fn resource_events_server_writes_report_a_new_record_edited_at_once_as_one_list_change() {
    let scratch = fixture();
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut listener = listen(&client, &["record://adr/0004"], true).await;

    new_adr(&client, "Use redis", BODY).await;
    set_description(&client, "adr/0004", "Redis, reworded.").await;

    assert_eq!(
        next_event(&mut listener, FIRST).await,
        Some(Event::ListChanged)
    );
    let rest = settled(&mut listener).await;
    assert!(rest.len() <= 1, "{rest:?}");
    assert!(rest
        .iter()
        .all(|event| *event == updated("record://adr/0004")));
}

#[tokio::test]
async fn resource_events_outside_edits_ignore_an_index_file_that_claims_a_record_type() {
    let scratch = fixture();
    write_record(
        &scratch,
        "adr/index.md",
        "ADR",
        "Index of decisions",
        "Accepted",
    );
    let client = connect(&scratch.bundle(), Era::Discover).await;
    let mut listener = listen(&client, &[ADR_ONE], true).await;

    write_record(
        &scratch,
        "adr/index.md",
        "ADR",
        "Index, rewritten",
        "Accepted",
    );

    assert_eq!(settled(&mut listener).await, vec![]);
}

/// Whether the process holds an inotify watch on `dir`; `None` where that
/// cannot be read.
#[cfg(target_os = "linux")]
fn watching(dir: &std::path::Path) -> Option<bool> {
    use std::os::unix::fs::MetadataExt;
    let inode = format!("ino:{:x} ", fs::metadata(dir).ok()?.ino());
    let found = fs::read_dir("/proc/self/fdinfo")
        .ok()?
        .filter_map(|entry| fs::read_to_string(entry.ok()?.path()).ok())
        .any(|info| {
            info.lines()
                .any(|line| line.starts_with("inotify ") && line.contains(&inode))
        });
    Some(found)
}

#[cfg(not(target_os = "linux"))]
fn watching(_: &std::path::Path) -> Option<bool> {
    None
}

/// Polls until `watching` reports `wanted`, for at most two seconds.
async fn watch_state_becomes(dir: &std::path::Path, wanted: bool) -> bool {
    let deadline = tokio::time::Instant::now() + FIRST;
    while tokio::time::Instant::now() < deadline {
        if watching(dir) == Some(wanted) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    false
}

#[tokio::test]
async fn resource_events_lifecycle_watch_the_bundle_only_while_a_listener_is_open() {
    let scratch = fixture();
    let bundle = scratch.bundle();
    if watching(&bundle).is_none() {
        eprintln!("skipped: watches are only observable through /proc on Linux");
        return;
    }
    let client = connect(&bundle, Era::Discover).await;
    new_adr(&client, "Use redis", BODY).await;
    assert_eq!(watching(&bundle), Some(false), "no listener, no watcher");

    let mut first = listen(&client, &[ADR_ONE], true).await;
    let mut second = listen(&client, &[ADR_TWO], true).await;
    assert!(
        watch_state_becomes(&bundle, true).await,
        "watching while listening"
    );

    first.cancel().await.expect("cancel");
    assert_eq!(watching(&bundle), Some(true), "one listener is left");
    second.cancel().await.expect("cancel");
    assert!(
        watch_state_becomes(&bundle, false).await,
        "released after the last cancel"
    );
}
