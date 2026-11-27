//! Change events for resource subscribers.
//!
//! While a listener asks for a resource notification, one task keeps the
//! fingerprints of the records in force. Two things wake it: a server write
//! (`Events::wrote`) and a file watcher on the bundle, coalesced by the
//! debouncer over [`WINDOW`]. Each wake rescans the bundle and reports only
//! the difference from the last scan, so a change seen by both is reported
//! once, and a file that is not a record reports nothing. With no such
//! listener there is no task and no watcher.

use crate::records;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use rmcp::model::{RequestId, SubscriptionFilter};
use rmcp::service::{SubscriptionContext, SubscriptionSink};
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

const WINDOW: Duration = Duration::from_millis(200);

/// Each record in force by URI, with a hash of its file.
type Seen = BTreeMap<String, u64>;

fn scan(docs_dir: &Path) -> Seen {
    records::entries(docs_dir)
        .into_iter()
        .map(|entry| {
            let mut hasher = DefaultHasher::new();
            entry.contents.hash(&mut hasher);
            (entry.uri, hasher.finish())
        })
        .collect()
}

/// What differs between two scans: whether the set of records changed, and
/// the URIs whose file changed.
struct Changes {
    list: bool,
    updated: Vec<String>,
}

fn changes(before: &Seen, after: &Seen) -> Changes {
    let updated = after
        .iter()
        .filter(|(uri, hash)| before.get(*uri).is_some_and(|old| old != *hash))
        .map(|(uri, _)| uri.clone())
        .collect();
    Changes {
        list: before.keys().ne(after.keys()),
        updated,
    }
}

fn wants_resources(filter: &SubscriptionFilter) -> bool {
    filter.resources_list_changed == Some(true)
        || filter
            .resource_subscriptions
            .as_ref()
            .is_some_and(|uris| !uris.is_empty())
}

#[derive(Clone)]
struct Listener {
    id: RequestId,
    accepted: SubscriptionFilter,
    sink: SubscriptionSink,
}

impl Listener {
    async fn tell(&self, changes: &Changes) {
        let list = changes.list && self.accepted.resources_list_changed == Some(true);
        let subscribed = self
            .accepted
            .resource_subscriptions
            .as_deref()
            .unwrap_or(&[]);
        if list {
            let _ = self.sink.notify_resource_list_changed().await;
        }
        for uri in changes
            .updated
            .iter()
            .filter(|uri| subscribed.contains(uri))
        {
            let _ = self.sink.notify_resource_updated(uri.as_str()).await;
        }
    }
}

/// The running task and the way to wake it.
struct Running {
    wake: UnboundedSender<()>,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct Shared {
    listeners: Vec<Listener>,
    running: Option<Running>,
}

fn locked(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The resource listeners of one bundle server.
#[derive(Clone)]
pub(crate) struct Events {
    docs_dir: PathBuf,
    shared: Arc<Mutex<Shared>>,
}

/// Keeps a listener registered; dropping it ends the listener, and the last
/// one stops the watcher.
struct Registration {
    id: RequestId,
    shared: Arc<Mutex<Shared>>,
}

impl Drop for Registration {
    fn drop(&mut self) {
        let mut shared = locked(&self.shared);
        shared.listeners.retain(|listener| listener.id != self.id);
        if shared.listeners.is_empty() {
            if let Some(running) = shared.running.take() {
                running.task.abort();
            }
        }
    }
}

impl Events {
    pub(crate) fn new(docs_dir: &Path) -> Self {
        Self {
            docs_dir: docs_dir.to_path_buf(),
            shared: Arc::default(),
        }
    }

    /// Holds one established listen until it is cancelled or the connection
    /// ends, delivering what it asked for while it lasts.
    pub(crate) async fn attend(&self, context: SubscriptionContext) {
        let _registration = wants_resources(context.accepted()).then(|| {
            self.join(Listener {
                id: context.sink().id().clone(),
                accepted: context.accepted().clone(),
                sink: context.sink().clone(),
            })
        });
        context.cancelled().await;
    }

    /// Wakes the task after a call that may have written to the bundle.
    pub(crate) fn wrote(&self) {
        if let Some(running) = &locked(&self.shared).running {
            let _ = running.wake.send(());
        }
    }

    fn join(&self, listener: Listener) -> Registration {
        let id = listener.id.clone();
        let mut shared = locked(&self.shared);
        shared.listeners.push(listener);
        if shared.running.is_none() {
            shared.running = Some(self.start());
        }
        Registration {
            id,
            shared: self.shared.clone(),
        }
    }

    fn start(&self) -> Running {
        let (wake, woken) = unbounded_channel();
        let watch = Watch {
            docs_dir: self.docs_dir.clone(),
            shared: self.shared.clone(),
            seen: scan(&self.docs_dir),
            watcher: watcher(&self.docs_dir, &wake),
            wake: wake.clone(),
        };
        Running {
            wake,
            task: tokio::spawn(watch.run(woken)),
        }
    }
}

/// A watcher on the bundle whose every result, an error included, wakes the
/// task; `None` while the bundle does not exist or the watcher cannot start,
/// when only server writes are seen.
fn watcher(docs_dir: &Path, wake: &UnboundedSender<()>) -> Option<Debouncer<RecommendedWatcher>> {
    if !docs_dir.is_dir() {
        return None;
    }
    let wake = wake.clone();
    let on_result = move |_: DebounceEventResult| {
        let _ = wake.send(());
    };
    let mut debouncer = new_debouncer(WINDOW, on_result).ok()?;
    debouncer
        .watcher()
        .watch(docs_dir, RecursiveMode::Recursive)
        .ok()?;
    Some(debouncer)
}

struct Watch {
    docs_dir: PathBuf,
    shared: Arc<Mutex<Shared>>,
    seen: Seen,
    watcher: Option<Debouncer<RecommendedWatcher>>,
    wake: UnboundedSender<()>,
}

impl Watch {
    async fn run(mut self, mut woken: UnboundedReceiver<()>) {
        while woken.recv().await.is_some() {
            while woken.try_recv().is_ok() {}
            if self.watcher.is_none() {
                self.watcher = watcher(&self.docs_dir, &self.wake);
            }
            let dir = self.docs_dir.clone();
            let Ok(now) = tokio::task::spawn_blocking(move || scan(&dir)).await else {
                continue;
            };
            let changes = changes(&self.seen, &now);
            self.seen = now;
            let listeners = locked(&self.shared).listeners.clone();
            for listener in &listeners {
                listener.tell(&changes).await;
            }
        }
    }
}
