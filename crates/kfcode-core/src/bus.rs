//! In-process publish/subscribe event bus backed by Tokio broadcast channels.
//!
//! Callers define events with [`BusEventDef`], publish payloads via [`Bus::publish`],
//! and receive them through either async callbacks or a [`tokio::sync::broadcast`] receiver.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// A serializable event payload carrying a type tag and arbitrary JSON properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusEvent {
    pub event_type: String,
    pub properties: serde_json::Value,
}

/// A static descriptor for a named event type, used as a key when publishing or subscribing.
pub struct BusEventDef {
    pub event_type: &'static str,
}

impl BusEventDef {
    /// Creates a new event definition with the given static event type string.
    pub const fn new(event_type: &'static str) -> Self {
        Self { event_type }
    }
}

type BoxedCallback = Box<dyn Fn(&str, serde_json::Value) + Send + Sync>;

struct Subscription {
    id: u64,
    callback: BoxedCallback,
}

/// An in-process event bus supporting typed subscriptions and a broadcast channel.
///
/// Subscribers can register callbacks per event type or for all events. A
/// [`tokio::sync::broadcast`] receiver is also available for channel-based consumers.
pub struct Bus {
    next_id: Arc<RwLock<u64>>,
    subscribers: Arc<RwLock<HashMap<String, Vec<Subscription>>>>,
    wildcard_subscribers: Arc<RwLock<Vec<Subscription>>>,
    tx: broadcast::Sender<BusEvent>,
}

impl Bus {
    /// Creates a new `Bus` with an internal broadcast channel of capacity 1024.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            next_id: Arc::new(RwLock::new(0)),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            wildcard_subscribers: Arc::new(RwLock::new(Vec::new())),
            tx,
        }
    }

    /// Publishes an event to all registered subscribers and the broadcast channel.
    ///
    /// Invokes every callback registered for `def.event_type` and every wildcard
    /// callback, then sends the event on the broadcast channel. Broadcast send
    /// errors (no active receivers) are silently ignored.
    pub async fn publish(&self, def: &BusEventDef, properties: serde_json::Value) {
        tracing::info!(event_type = def.event_type, "publishing event");

        let event = BusEvent {
            event_type: def.event_type.to_string(),
            properties: properties.clone(),
        };

        let _ = self.tx.send(event.clone());

        let subscribers = self.subscribers.read().await;
        if let Some(subs) = subscribers.get(def.event_type) {
            for sub in subs {
                (sub.callback)(def.event_type, properties.clone());
            }
        }

        let wildcard = self.wildcard_subscribers.read().await;
        for sub in wildcard.iter() {
            (sub.callback)(def.event_type, properties.clone());
        }
    }

    /// Registers a callback for a specific event type and returns its subscription ID.
    ///
    /// The returned ID can be passed to [`Bus::unsubscribe`] to remove the callback.
    pub async fn subscribe<F>(&self, def: &BusEventDef, callback: F) -> u64
    where
        F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        let id = {
            let mut next = self.next_id.write().await;
            *next += 1;
            *next
        };

        let mut subscribers = self.subscribers.write().await;
        let subs = subscribers
            .entry(def.event_type.to_string())
            .or_insert_with(Vec::new);
        subs.push(Subscription {
            id,
            callback: Box::new(callback),
        });

        id
    }

    /// Removes the subscription with the given ID from the named event type.
    pub async fn unsubscribe(&self, event_type: &str, id: u64) {
        let mut subscribers = self.subscribers.write().await;
        if let Some(subs) = subscribers.get_mut(event_type) {
            subs.retain(|s| s.id != id);
        }
    }

    /// Registers a wildcard callback that receives every published event and returns its subscription ID.
    ///
    /// The returned ID can be passed to [`Bus::unsubscribe_all`] to remove the callback.
    pub async fn subscribe_all<F>(&self, callback: F) -> u64
    where
        F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        let id = {
            let mut next = self.next_id.write().await;
            *next += 1;
            *next
        };

        let mut wildcard = self.wildcard_subscribers.write().await;
        wildcard.push(Subscription {
            id,
            callback: Box::new(callback),
        });

        id
    }

    /// Removes the wildcard subscription with the given ID.
    pub async fn unsubscribe_all(&self, id: u64) {
        let mut wildcard = self.wildcard_subscribers.write().await;
        wildcard.retain(|s| s.id != id);
    }

    /// Returns a new broadcast receiver that will receive all future published events.
    pub fn subscribe_channel(&self) -> broadcast::Receiver<BusEvent> {
        self.tx.subscribe()
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a [`BusEventDef`] with the given static event type string.
///
/// Convenience wrapper around [`BusEventDef::new`] for use in `const` or static contexts.
pub fn define_event(event_type: &'static str) -> BusEventDef {
    BusEventDef::new(event_type)
}
