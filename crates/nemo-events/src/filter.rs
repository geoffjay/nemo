//! Event filtering for subscriptions.

use crate::Event;
use std::sync::Arc;

/// Filter for selecting which events a subscription receives.
#[derive(Clone)]
pub enum EventFilter {
    /// Match any event (no filtering).
    Any,
    /// Match exact event type.
    Type(String),
    /// Match event type prefix (e.g., "data." matches "data.updated").
    Prefix(String),
    /// Match using glob pattern (e.g., "data.*.error").
    Pattern(String),
    /// Match events from a specific source.
    Source(String),
    /// Combine filters with AND (all must match).
    And(Vec<EventFilter>),
    /// Combine filters with OR (any must match).
    Or(Vec<EventFilter>),
    /// Negate a filter.
    Not(Box<EventFilter>),
    /// Custom predicate function.
    Custom(Arc<dyn Fn(&Event) -> bool + Send + Sync>),
}

impl std::fmt::Debug for EventFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Any => write!(f, "Any"),
            Self::Type(t) => write!(f, "Type({:?})", t),
            Self::Prefix(p) => write!(f, "Prefix({:?})", p),
            Self::Pattern(p) => write!(f, "Pattern({:?})", p),
            Self::Source(s) => write!(f, "Source({:?})", s),
            Self::And(filters) => write!(f, "And({:?})", filters),
            Self::Or(filters) => write!(f, "Or({:?})", filters),
            Self::Not(filter) => write!(f, "Not({:?})", filter),
            Self::Custom(_) => write!(f, "Custom(<fn>)"),
        }
    }
}

impl EventFilter {
    /// Check if an event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        match self {
            Self::Any => true,
            Self::Type(t) => event.event_type == *t,
            Self::Prefix(p) => event.event_type.starts_with(p),
            Self::Pattern(p) => glob_match(p, &event.event_type),
            Self::Source(s) => event.source.as_ref() == Some(s),
            Self::And(filters) => filters.iter().all(|f| f.matches(event)),
            Self::Or(filters) => filters.iter().any(|f| f.matches(event)),
            Self::Not(filter) => !filter.matches(event),
            Self::Custom(predicate) => predicate(event),
        }
    }

    /// Combine this filter with another using AND.
    pub fn and(self, other: EventFilter) -> Self {
        match self {
            Self::And(mut filters) => {
                filters.push(other);
                Self::And(filters)
            }
            _ => Self::And(vec![self, other]),
        }
    }

    /// Combine this filter with another using OR.
    pub fn or(self, other: EventFilter) -> Self {
        match self {
            Self::Or(mut filters) => {
                filters.push(other);
                Self::Or(filters)
            }
            _ => Self::Or(vec![self, other]),
        }
    }
}

/// Simple glob matching for event type patterns.
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let text_parts: Vec<&str> = text.split('.').collect();
    glob_match_parts(&pattern_parts, &text_parts)
}

fn glob_match_parts(pattern: &[&str], text: &[&str]) -> bool {
    match (pattern.first(), text.first()) {
        (None, None) => true,
        (None, Some(_)) => false,
        (Some(&"**"), _) => {
            glob_match_parts(&pattern[1..], text)
                || (!text.is_empty() && glob_match_parts(pattern, &text[1..]))
        }
        (Some(&"*"), Some(_)) => glob_match_parts(&pattern[1..], &text[1..]),
        (Some(&"*"), None) => false,
        (Some(p), Some(t)) if *p == *t => glob_match_parts(&pattern[1..], &text[1..]),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_event(event_type: &str) -> Event {
        Event::new(event_type, json!(null))
    }

    #[test]
    fn test_any_filter() {
        let filter = EventFilter::Any;
        assert!(filter.matches(&make_event("anything")));
    }

    #[test]
    fn test_type_filter() {
        let filter = EventFilter::Type("data.updated".to_string());
        assert!(filter.matches(&make_event("data.updated")));
        assert!(!filter.matches(&make_event("data.error")));
    }

    #[test]
    fn test_prefix_filter() {
        let filter = EventFilter::Prefix("data.".to_string());
        assert!(filter.matches(&make_event("data.updated")));
        assert!(filter.matches(&make_event("data.error")));
        assert!(!filter.matches(&make_event("ui.notification")));
    }

    #[test]
    fn test_glob_pattern() {
        assert!(glob_match("data.*", "data.updated"));
        assert!(glob_match("data.*", "data.error"));
        assert!(!glob_match("data.*", "data.foo.bar"));
        assert!(glob_match("data.**", "data.updated"));
        assert!(glob_match("data.**", "data.foo.bar"));
    }

    #[test]
    fn test_and_filter() {
        let filter = EventFilter::Prefix("data.".to_string())
            .and(EventFilter::Not(Box::new(EventFilter::Type(
                "data.heartbeat".to_string(),
            ))));

        assert!(filter.matches(&make_event("data.updated")));
        assert!(!filter.matches(&make_event("data.heartbeat")));
        assert!(!filter.matches(&make_event("ui.notification")));
    }
}
