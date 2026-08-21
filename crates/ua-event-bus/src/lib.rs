//! ua-event-bus — in-memory event bus for unified-agent-rs.
//!
//! A lightweight publish/subscribe bus. `EventBus::subscribe` returns a
//! `SubscriptionId`; the bus calls the matching `Handler` on `publish`.
//! Handlers run synchronously in publish-order; panics in a handler are
//! caught and recorded as `DispatchError::HandlerPanic`.
//!
//! Use for cross-crate signaling: telemetry flush, skill-invalidation,
//! memory compaction, fs-state checkpoint.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Opaque subscription id.
pub type SubscriptionId = u64;

/// A single event: named topic + JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub topic: String,
    pub payload: serde_json::Value,
}

impl Event {
    pub fn new(topic: impl Into<String>, payload: serde_json::Value) -> Self {
        Self { topic: topic.into(), payload }
    }

    /// Convenience: empty payload event.
    pub fn signal(topic: impl Into<String>) -> Self {
        Self::new(topic, serde_json::Value::Null)
    }
}

/// Handler invoked for each matching event.
pub type Handler = Box<dyn Fn(&Event) -> Result<(), HandlerError> + Send + Sync>;

/// Errors a handler can report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerError {
    /// Handler explicitly rejected the event.
    Rejected(String),
    /// Handler panicked — caught by the bus.
    HandlerPanic,
}

/// Dispatch outcome for a single subscriber.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchResult {
    /// Handler returned Ok.
    Ok,
    /// Handler returned Err.
    Failed(HandlerError),
    /// No handler was registered for this topic.
    NoSubscriber,
}

/// In-memory event bus. Clone shares the underlying state.
#[derive(Clone, Default)]
pub struct EventBus {
    inner: Arc<Mutex<BusInner>>,
}

#[derive(Default)]
struct BusInner {
    next_id: SubscriptionId,
    /// topic → list of (id, handler)
    subs: HashMap<String, Vec<(SubscriptionId, Handler)>>,
    /// Counters for metrics.
    published: u64,
    dispatched: u64,
    failed: u64,
}

impl EventBus {
    /// Subscribe to a topic. Returns the new SubscriptionId.
    pub fn subscribe<F>(&self, topic: impl Into<String>, handler: F) -> SubscriptionId
    where
        F: Fn(&Event) -> Result<(), HandlerError> + Send + Sync + 'static,
    {
        let topic = topic.into();
        let mut inner = self.inner.lock().unwrap();
        let id = inner.next_id;
        inner.next_id += 1;
        inner.subs.entry(topic).or_default().push((id, Box::new(handler)));
        id
    }

    /// Unsubscribe by id. Returns true if a subscription was removed.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let mut inner = self.inner.lock().unwrap();
        for subs in inner.subs.values_mut() {
            let before = subs.len();
            subs.retain(|(sid, _)| *sid != id);
            if subs.len() < before {
                return true;
            }
        }
        false
    }

    /// Publish an event. Returns one DispatchResult per subscriber, in
    /// subscription order. Empty vec means no subscribers.
    pub fn publish(&self, event: &Event) -> Vec<(SubscriptionId, DispatchResult)> {
        let mut inner = self.inner.lock().unwrap();
        inner.published += 1;
        let subs = inner.subs.get(&event.topic).cloned().unwrap_or_default();
        drop(inner);

        let mut out = Vec::with_capacity(subs.len());
        for (id, handler) in subs {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler(event)
            }));
            let result = match result {
                Ok(Ok(())) => DispatchResult::Ok,
                Ok(Err(e)) => DispatchResult::Failed(e),
                Err(_) => DispatchResult::Failed(HandlerError::HandlerPanic),
            };
            out.push((id, result));
        }

        let mut inner = self.inner.lock().unwrap();
        for (_, r) in &out {
            inner.dispatched += 1;
            if matches!(r, DispatchResult::Failed(_)) {
                inner.failed += 1;
            }
        }
        out
    }

    /// Number of active subscriptions for a topic.
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.inner.lock().unwrap().subs.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    /// Total active subscriptions across all topics.
    pub fn total_subscriptions(&self) -> usize {
        self.inner.lock().unwrap().subs.values().map(|v| v.len()).sum()
    }

    /// Metrics snapshot.
    pub fn metrics(&self) -> BusMetrics {
        let inner = self.inner.lock().unwrap();
        BusMetrics {
            published: inner.published,
            dispatched: inner.dispatched,
            failed: inner.failed,
            subscriptions: inner.subs.values().map(|v| v.len()).sum(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusMetrics {
    pub published: u64,
    pub dispatched: u64,
    pub failed: u64,
    pub subscriptions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn subscribe_and_publish() {
        let bus = EventBus::default();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        bus.subscribe("telemetry.flush", move |_| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let results = bus.publish(&Event::signal("telemetry.flush"));
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].1, DispatchResult::Ok));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn publish_no_subscribers() {
        let bus = EventBus::default();
        let results = bus.publish(&Event::signal("nope"));
        assert!(results.is_empty());
    }

    #[test]
    fn multiple_subscribers_dispatched_in_order() {
        let bus = EventBus::default();
        let order = Arc::new(Mutex::new(Vec::new()));
        let o1 = order.clone();
        let o2 = order.clone();
        let id1 = bus.subscribe("x", move |_| { o1.lock().unwrap().push(1); Ok(()) });
        let id2 = bus.subscribe("x", move |_| { o2.lock().unwrap().push(2); Ok(()) });
        let _ = bus.publish(&Event::signal("x"));
        let order = order.lock().unwrap();
        assert_eq!(*order, vec![1, 2]);
        assert_ne!(id1, id2);
    }

    #[test]
    fn handler_failure_recorded() {
        let bus = EventBus::default();
        bus.subscribe("fail", |_| Err(HandlerError::Rejected("bad".into())));
        let results = bus.publish(&Event::signal("fail"));
        assert!(matches!(results[0].1, DispatchResult::Failed(HandlerError::Rejected(_))));
    }

    #[test]
    fn handler_panic_caught() {
        let bus = EventBus::default();
        bus.subscribe("panic", |_| panic!("boom"));
        let results = bus.publish(&Event::signal("panic"));
        assert!(matches!(results[0].1, DispatchResult::Failed(HandlerError::HandlerPanic)));
    }

    #[test]
    fn unsubscribe_removes_handler() {
        let bus = EventBus::default();
        let id = bus.subscribe("x", |_| Ok(()));
        assert_eq!(bus.subscriber_count("x"), 1);
        assert!(bus.unsubscribe(id));
        assert_eq!(bus.subscriber_count("x"), 0);
    }

    #[test]
    fn unsubscribe_unknown_id_returns_false() {
        let bus = EventBus::default();
        assert!(!bus.unsubscribe(999));
    }

    #[test]
    fn clone_shares_state() {
        let bus1 = EventBus::default();
        let bus2 = bus1.clone();
        bus1.subscribe("shared", |_| Ok(()));
        assert_eq!(bus2.subscriber_count("shared"), 1);
    }

    #[test]
    fn metrics_track_publish_and_failures() {
        let bus = EventBus::default();
        bus.subscribe("ok", |_| Ok(()));
        bus.subscribe("err", |_| Err(HandlerError::Rejected("x".into())));
        bus.publish(&Event::signal("ok"));
        bus.publish(&Event::signal("err"));
        bus.publish(&Event::signal("none"));
        let m = bus.metrics();
        assert_eq!(m.published, 3);
        assert_eq!(m.dispatched, 2);
        assert_eq!(m.failed, 1);
        assert_eq!(m.subscriptions, 2);
    }

    #[test]
    fn event_with_payload_roundtrip() {
        let e = Event::new("topic", serde_json::json!({"k": "v"}));
        let s = serde_json::to_string(&e).unwrap();
        let back: Event = serde_json::from_str(&s).unwrap();
        assert_eq!(e, back);
    }
}
