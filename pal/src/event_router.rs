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
    receiver: Option<crossbeam_channel::Receiver<UIEvent>>,
}

impl EventRouter {
    /// Create a new event router
    pub fn new() -> Self {
        Self {
            callback: None,
            receiver: None,
        }
    }

    /// Set the channel receiver for injected events (e.g. from RPC server)
    pub fn set_event_receiver(&mut self, receiver: crossbeam_channel::Receiver<UIEvent>) {
        self.receiver = Some(receiver);
    }

    /// Set the callback that will receive all routed events
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut(UIEvent) + Send + 'static,
    {
        self.callback = Some(Box::new(callback));
    }

    /// Route an event to the registered callback
    pub fn route_event(&mut self, event: UIEvent) {
        if let Some(ref mut callback) = self.callback {
            callback(event);
        }
    }

    /// Poll for injected events from the receiver and route them
    /// Should be called periodically on the main thread
    pub fn poll_events(&mut self) {
        if let Some(ref receiver) = self.receiver {
            let rx = receiver.clone(); // Clone receiver to avoid borrowing self
            while let Ok(event) = rx.try_recv() {
                self.route_event(event);
            }
        }
    }

    /// Inject an event programmatically (Legacy/Direct)
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
