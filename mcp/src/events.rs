//! Change events for resource subscribers.
//!
//! While a listener (a `subscriptions/listen` stream or, through
//! `legacy_subscriptions`, a `resources/subscribe` session) asks for a
//! resource notification, one task keeps the
//! fingerprints of the records in force. Two things wake it: a server write
//! (`Events::wrote`) and a file watcher on the bundle. Both coalesce over
//! [`WINDOW`]: the debouncer for the watcher, a wait in the task for a write.
//! Each wake rescans the bundle and reports only the difference from the last
//! scan, so a change seen by both is reported once, and a file that is not a
//! record reports nothing. With no such listener there is no task and no
//! watcher.

use crate::legacy_subscriptions::{Channel, Key};
use crate::records;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use rmcp::model::SubscriptionFilter;
use rmcp::service::SubscriptionContext;
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
pub(crate) struct Listener {
    pub(crate) key: Key,
    pub(crate) accepted: SubscriptionFilter,
    pub(crate) channel: Channel,
}

impl Listener {
    /// Sends what this listener asked for; false once its channel has closed.
    async fn tell(&self, changes: &Changes) -> bool {
        let list = changes.list && self.accepted.resources_list_changed == Some(true);
        let subscribed = self
            .accepted
            .resource_subscriptions
            .as_deref()
            .unwrap_or(&[]);
        let mut open = !list || self.channel.list_changed().await;
        for uri in changes
            .updated
            .iter()
            .filter(|uri| subscribed.contains(uri))
        {
            open &= self.channel.updated(uri).await;
        }
        open
    }
}

/// Why the task woke. A watcher result has already waited out [`WINDOW`] in
/// the debouncer; a server write has not, so the task waits it out before it
/// scans and a burst of writes is reported once.
enum Wake {
    Write,
    Watched,
}

/// The running task and the way to wake it.
struct Running {
    wake: UnboundedSender<Wake>,
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
    key: Key,
    shared: Arc<Mutex<Shared>>,
}

impl Drop for Registration {
    fn drop(&mut self) {
        depart(&self.shared, |listeners| {
            listeners.retain(|listener| listener.key != self.key);
        });
    }
}

/// Removes listeners with `leave`; the last one out stops the task.
fn depart(shared: &Mutex<Shared>, leave: impl FnOnce(&mut Vec<Listener>)) {
    let mut shared = locked(shared);
    leave(&mut shared.listeners);
    if shared.listeners.is_empty() {
        if let Some(running) = shared.running.take() {
            running.task.abort();
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
            let key = Key::Listen(context.sink().id().clone());
            let listener = Listener {
                key: key.clone(),
                accepted: context.accepted().clone(),
                channel: Channel::Listen(context.sink().clone()),
            };
            self.register(|listeners| listeners.push(listener));
            Registration {
                key,
                shared: self.shared.clone(),
            }
        });
        context.cancelled().await;
    }

    /// Ends the listeners `leave` removes, and the task with the last one.
    pub(crate) fn depart(&self, leave: impl FnOnce(&mut Vec<Listener>)) {
        depart(&self.shared, leave);
    }

    /// Wakes the task after a call that may have written to the bundle.
    pub(crate) fn wrote(&self) {
        if let Some(running) = &locked(&self.shared).running {
            let _ = running.wake.send(Wake::Write);
        }
    }

    /// Adds listeners with `add` and, for the first one, starts the task. The
    /// first scan runs with the lock released, so a second listener never
    /// waits on it, and without yielding, so the baseline is taken before the
    /// client's next request is served; a blocking-pool scan would race an
    /// edit made right after the listen is acknowledged and miss it.
    pub(crate) fn register(&self, add: impl FnOnce(&mut Vec<Listener>)) {
        let first = {
            let mut shared = locked(&self.shared);
            add(&mut shared.listeners);
            shared.running.is_none()
        };
        if first {
            let running = self.start();
            let mut shared = locked(&self.shared);
            match shared.running {
                None => shared.running = Some(running),
                Some(_) => running.task.abort(),
            }
        }
    }

    /// Builds the watcher before the first scan, so an edit between the two
    /// is still reported.
    fn start(&self) -> Running {
        let (wake, woken) = unbounded_channel();
        let watcher = watcher(&self.docs_dir, &wake);
        let watch = Watch {
            docs_dir: self.docs_dir.clone(),
            shared: self.shared.clone(),
            seen: scan(&self.docs_dir),
            watcher,
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
fn watcher(docs_dir: &Path, wake: &UnboundedSender<Wake>) -> Option<Debouncer<RecommendedWatcher>> {
    if !docs_dir.is_dir() {
        return None;
    }
    let wake = wake.clone();
    let on_result = move |_: DebounceEventResult| {
        let _ = wake.send(Wake::Watched);
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
    wake: UnboundedSender<Wake>,
}

impl Watch {
    async fn run(mut self, mut woken: UnboundedReceiver<Wake>) {
        while let Some(wake) = woken.recv().await {
            if matches!(wake, Wake::Write) {
                tokio::time::sleep(WINDOW).await;
            }
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
            self.tell_all(&changes).await;
        }
    }

    /// Tells every listener, then drops those whose channel has closed.
    async fn tell_all(&self, changes: &Changes) {
        let listeners = locked(&self.shared).listeners.clone();
        let mut closed = Vec::new();
        for listener in &listeners {
            if !listener.tell(changes).await {
                closed.push(listener.key.clone());
            }
        }
        if !closed.is_empty() {
            depart(&self.shared, |all| all.retain(|l| !closed.contains(&l.key)));
        }
    }
}
