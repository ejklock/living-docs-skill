//! `resources/subscribe` for a client on the `initialize` protocol revision.
//!
//! Such a client has no `subscriptions/listen` stream. Its subscriptions join
//! the same listeners the events task already serves, one listener per session
//! that holds its subscribed URIs, so one watcher and one scan serve both
//! kinds. A session with at least one subscription also hears of a changed
//! record list. A session ends its listener by unsubscribing from every URI or
//! by closing: a notification that cannot be sent retires it.

use crate::events::{Events, Listener};
use rmcp::model::{
    ErrorData, InitializeRequestParams, RequestId, ResourceUpdatedNotificationParam,
    SubscriptionFilter,
};
use rmcp::service::{Peer, SubscriptionSink};
use rmcp::RoleServer;
use std::sync::Arc;

/// Identifies a listener: a `subscriptions/listen` request, or a legacy
/// session by the handshake its peer completed.
#[derive(Clone)]
pub(crate) enum Key {
    Listen(RequestId),
    Session(Arc<InitializeRequestParams>),
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Listen(a), Self::Listen(b)) => a == b,
            (Self::Session(a), Self::Session(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Key {
    /// The session's key; none for a peer that has not completed the
    /// handshake, which no other session can be told apart from.
    fn session(peer: &Peer<RoleServer>) -> Option<Self> {
        peer.peer_info().map(Self::Session)
    }
}

/// Where a listener's notifications go.
#[derive(Clone)]
pub(crate) enum Channel {
    Listen(SubscriptionSink),
    Session(Peer<RoleServer>),
}

impl Channel {
    /// Whether the notification was sent. A `subscriptions/listen` stream is
    /// ended by its own cancellation, so only a closed session reports false.
    pub(crate) async fn list_changed(&self) -> bool {
        match self {
            Self::Listen(sink) => {
                let _ = sink.notify_resource_list_changed().await;
                true
            }
            Self::Session(peer) => peer.notify_resource_list_changed().await.is_ok(),
        }
    }

    pub(crate) async fn updated(&self, uri: &str) -> bool {
        match self {
            Self::Listen(sink) => {
                let _ = sink.notify_resource_updated(uri).await;
                true
            }
            Self::Session(peer) => {
                let param = ResourceUpdatedNotificationParam::new(uri);
                peer.notify_resource_updated(param).await.is_ok()
            }
        }
    }
}

fn session_listener(key: Key, peer: Peer<RoleServer>, uri: &str) -> Listener {
    let accepted = SubscriptionFilter::builder()
        .resources_list_changed()
        .resource_subscriptions([uri])
        .build();
    Listener {
        key,
        accepted,
        channel: Channel::Session(peer),
    }
}

fn subscribed(listener: &mut Listener) -> &mut Vec<String> {
    listener
        .accepted
        .resource_subscriptions
        .get_or_insert_with(Vec::new)
}

impl Events {
    /// Subscribes the session behind `peer` to `uri`, starting the watcher if
    /// this is the first listener of any kind. A peer that has not completed
    /// the handshake is refused: it has no identity to hold a listener by.
    pub(crate) fn subscribe(&self, peer: &Peer<RoleServer>, uri: &str) -> Result<(), ErrorData> {
        let Some(key) = Key::session(peer) else {
            return Err(ErrorData::invalid_request(
                "subscribe needs a completed initialize handshake",
                None,
            ));
        };
        self.register(|listeners| {
            match listeners.iter_mut().find(|listener| listener.key == key) {
                Some(listener) => {
                    let uris = subscribed(listener);
                    if !uris.iter().any(|known| known == uri) {
                        uris.push(uri.to_string());
                    }
                }
                None => listeners.push(session_listener(key.clone(), peer.clone(), uri)),
            }
        });
        Ok(())
    }

    /// Drops `uri` from the session's subscriptions; the session's listener
    /// ends with its last one, and the last listener stops the watcher.
    pub(crate) fn unsubscribe(&self, peer: &Peer<RoleServer>, uri: &str) {
        let Some(key) = Key::session(peer) else {
            return;
        };
        self.depart(|listeners| {
            for listener in listeners.iter_mut().filter(|l| l.key == key) {
                subscribed(listener).retain(|known| known != uri);
            }
            listeners.retain_mut(|l| l.key != key || !subscribed(l).is_empty());
        });
    }
}
