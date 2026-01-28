//! Widget identifier system for debugging and RPC testing

use std::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for a widget instance
///
/// Each widget is assigned a unique ID on creation using an atomic counter.
/// IDs are used for:
/// - RPC queries to get/set widget state
/// - Debugging and logging
/// - Future serialization of UI layouts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    /// Create a new unique widget ID
    ///
    /// Uses an atomic counter to ensure uniqueness across threads.
    /// IDs start at 1 (0 is reserved for invalid/null).
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        WidgetId(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Create a WidgetId from a raw u64 value
    ///
    /// This is useful for deserializing IDs from RPC messages or saved layouts.
    pub fn from_raw(id: u64) -> Self {
        WidgetId(id)
    }

    /// Get the raw u64 value of this ID
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Check if this is a valid ID (non-zero)
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for WidgetId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WidgetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Widget#{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_id_uniqueness() {
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();
        let id3 = WidgetId::new();

        // All IDs should be different
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);

        // All IDs should be valid
        assert!(id1.is_valid());
        assert!(id2.is_valid());
        assert!(id3.is_valid());
    }

    #[test]
    fn test_widget_id_from_raw() {
        let id = WidgetId::from_raw(42);
        assert_eq!(id.as_u64(), 42);
        assert!(id.is_valid());

        let invalid = WidgetId::from_raw(0);
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_widget_id_display() {
        let id = WidgetId::from_raw(123);
        assert_eq!(format!("{}", id), "Widget#123");
    }

    #[test]
    fn test_widget_id_ordering() {
        // IDs should increment
        let id1 = WidgetId::new();
        let id2 = WidgetId::new();
        assert!(id2.as_u64() > id1.as_u64());
    }
}
