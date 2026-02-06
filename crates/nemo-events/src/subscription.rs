//! Event subscription management.

use crate::{Event, EventFilter};
use futures::{Future, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::broadcast;

/// A subscription to events from the event bus.
pub struct EventSubscription {
    rx: broadcast::Receiver<Event>,
    filter: EventFilter,
}

impl EventSubscription {
    /// Create a new subscription with the given receiver and filter.
    pub(crate) fn new(rx: broadcast::Receiver<Event>, filter: EventFilter) -> Self {
        Self { rx, filter }
    }

    /// Receive the next matching event.
    /// Returns None if the channel is closed.
    pub async fn recv(&mut self) -> Option<Event> {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Some(event);
                    }
                }
                Err(broadcast::error::RecvError::Closed) => return None,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Event subscription lagged, missed {} events", n);
                }
            }
        }
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&mut self) -> Result<Option<Event>, SubscriptionClosed> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => {
                    if self.filter.matches(&event) {
                        return Ok(Some(event));
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => return Ok(None),
                Err(broadcast::error::TryRecvError::Closed) => return Err(SubscriptionClosed),
                Err(broadcast::error::TryRecvError::Lagged(n)) => {
                    tracing::warn!("Event subscription lagged, missed {} events", n);
                }
            }
        }
    }

    /// Convert this subscription into a Stream.
    pub fn into_stream(self) -> EventStream {
        EventStream { subscription: self }
    }
}

/// Error indicating the subscription channel is closed.
#[derive(Debug, Clone, Copy)]
pub struct SubscriptionClosed;

impl std::fmt::Display for SubscriptionClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "subscription closed")
    }
}

impl std::error::Error for SubscriptionClosed {}

/// A Stream adapter for EventSubscription.
pub struct EventStream {
    subscription: EventSubscription,
}

impl Stream for EventStream {
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let fut = self.subscription.recv();
        tokio::pin!(fut);
        fut.poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_subscription_receives_events() {
        let (tx, rx) = broadcast::channel(16);
        let mut sub = EventSubscription::new(rx, EventFilter::Any);

        tx.send(Event::new("test", json!({"value": 1}))).unwrap();

        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, "test");
    }

    #[tokio::test]
    async fn test_subscription_filters_events() {
        let (tx, rx) = broadcast::channel(16);
        let mut sub = EventSubscription::new(rx, EventFilter::Type("wanted".to_string()));

        tx.send(Event::new("unwanted", json!(null))).unwrap();
        tx.send(Event::new("wanted", json!(null))).unwrap();

        let event = sub.recv().await.unwrap();
        assert_eq!(event.event_type, "wanted");
    }

    #[tokio::test]
    async fn test_subscription_returns_none_on_close() {
        let (tx, rx) = broadcast::channel::<Event>(16);
        let mut sub = EventSubscription::new(rx, EventFilter::Any);

        drop(tx);

        assert!(sub.recv().await.is_none());
    }
}
