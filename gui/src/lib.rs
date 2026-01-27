//! # GUI Framework
//!
//! Shared GUI components used by both the plugin and standalone binary.
//!
//! ## Architecture
//!
//! This crate orchestrates the PAL (windowing) and render (Vulkan) layers to
//! provide a complete UI framework. It handles:
//!
//! - Window lifecycle management
//! - Event routing (mouse, keyboard)
//! - Render loop coordination
//! - UI state management
//!
//! ## Usage
//!
//! Both the plugin and standalone binary use this crate to avoid code duplication.
//! The key difference is:
//!
//! - **Plugin**: Receives parent window handle from host via VST3/CLAP attach()
//! - **Standalone**: Creates its own top-level window
//!
//! Both then use the same rendering and event handling code.

use anyhow::Result;

/// UI Event types for input handling
#[derive(Debug, Clone, Copy)]
pub enum UIEvent {
    /// Mouse button pressed
    MouseDown { x: f64, y: f64, button: MouseButton },

    /// Mouse button released
    MouseUp { x: f64, y: f64, button: MouseButton },

    /// Mouse moved
    MouseMove { x: f64, y: f64 },

    /// Key pressed
    KeyDown { key: Key, modifiers: Modifiers },

    /// Key released
    KeyUp { key: Key, modifiers: Modifiers },

    /// Window resized
    Resize { width: u32, height: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    // TODO: Phase 2 - Add key codes
    Unknown,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

/// Main GUI context
pub struct GuiContext {
    #[cfg(target_os = "macos")]
    window: pal::MacOSWindow,

    #[cfg(target_os = "windows")]
    window: pal::Win32Window,
    // TODO: Phase 3 - Add render context
}

impl GuiContext {
    /// Create a new GUI context attached to a parent window (plugin mode)
    pub unsafe fn attach(parent: *mut std::ffi::c_void, _width: u32, _height: u32) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            use pal::NativeWindow;
            let window = pal::MacOSWindow::attach(parent)?;
            Ok(GuiContext { window })
        }

        #[cfg(target_os = "windows")]
        {
            use pal::NativeWindow;
            let window = pal::Win32Window::attach(parent)?;
            Ok(GuiContext { window })
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            anyhow::bail!("Unsupported platform")
        }
    }

    /// Create a new standalone GUI context (standalone mode)
    #[allow(unreachable_code)]
    pub fn create_standalone(_width: u32, _height: u32, _title: &str) -> Result<Self> {
        // TODO: Phase 2.5 - Create top-level window
        // For now, this is not implemented
        anyhow::bail!("Standalone mode not yet implemented")

        // The actual implementation will:
        // 1. Create NSWindow/HWND
        // 2. Create NSView/child HWND
        // 3. Use same PAL code via attach()
    }

    /// Process a UI event
    pub fn handle_event(&mut self, _event: UIEvent) -> Result<()> {
        // TODO: Phase 2.5 - Route events to appropriate handlers
        Ok(())
    }

    /// Render a frame
    pub fn render(&mut self) -> Result<()> {
        // TODO: Phase 3 - Vulkan rendering
        Ok(())
    }

    /// Get the raw window handle for Vulkan surface creation
    pub fn raw_handle(&self) -> raw_window_handle::RawWindowHandle {
        use pal::NativeWindow;
        self.window.get_raw_handle()
    }

    /// Check if window is visible
    pub fn is_visible(&self) -> bool {
        use pal::NativeWindow;
        self.window.is_visible()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_event_creation() {
        let event = UIEvent::MouseDown {
            x: 100.0,
            y: 200.0,
            button: MouseButton::Left,
        };

        match event {
            UIEvent::MouseDown { x, y, button } => {
                assert_eq!(x, 100.0);
                assert_eq!(y, 200.0);
                assert_eq!(button, MouseButton::Left);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
