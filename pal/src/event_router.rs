//! Event routing middleware
//!
//! Provides an abstraction layer for routing UI events from multiple sources
//! (hardware events from NSView, injected events from RPC server) to a single
//! event handler callback.

use crate::UIEvent;

/// Event router that accepts events from multiple sources and routes them
/// to a registered callback.
///
/// This allows both hardware events (mouse, keyboard) and programmatically
/// injected events (from RPC debug server) to be handled uniformly.
pub struct EventRouter {
    callback: Option<Box<dyn FnMut(UIEvent) + Send>>,
}

impl EventRouter {
    /// Create a new event router with no callback registered
    pub fn new() -> Self {
        Self { callback: None }
    }

    /// Set the callback that will receive all routed events
    ///
    /// # Thread Safety
    /// This must be called on the main thread. The callback will be invoked
    /// on the main thread when events are routed.
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut(UIEvent) + Send + 'static,
    {
        self.callback = Some(Box::new(callback));
    }

    /// Route an event to the registered callback
    ///
    /// This is called by platform-specific event handlers (e.g., NSView methods)
    /// to deliver hardware events.
    pub fn route_event(&mut self, event: UIEvent) {
        if let Some(ref mut callback) = self.callback {
            callback(event);
        }
    }

    /// Inject an event programmatically
    ///
    /// This is intended for use by the RPC debug server (Phase 5) to simulate
    /// user input for automated testing.
    ///
    /// # Thread Safety
    /// This must be called on the main thread to avoid race conditions with
    /// hardware event delivery.
    pub fn inject_event(&mut self, event: UIEvent) {
        self.route_event(event);
    }
}

impl Default for EventRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_router_no_callback() {
        let mut router = EventRouter::new();
        // Should not panic when no callback is set
        router.route_event(UIEvent::MouseDown {
            x: 100.0,
            y: 200.0,
            button: 0,
        });
    }

    #[test]
    fn test_event_router_with_callback() {
        let mut router = EventRouter::new();

        router.set_callback(move |_event| {
            // Callback invoked successfully
        });

        router.route_event(UIEvent::MouseDown {
            x: 150.0,
            y: 250.0,
            button: 1,
        });

        // Test passes if no panic occurs
    }

    #[test]
    fn test_inject_event() {
        let mut router = EventRouter::new();

        router.set_callback(move |_event| {
            // Callback invoked successfully
        });

        // Simulate RPC injection
        router.inject_event(UIEvent::KeyDown { keycode: 65 });
        router.inject_event(UIEvent::KeyUp { keycode: 65 });

        // Test passes if no panic occurs
    }
}
