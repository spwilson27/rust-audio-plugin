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

// Vulkan rendering backend
pub mod fonts;
pub mod vulkan;

// Widget framework
pub mod widgets;

// Clipboard support
pub mod clipboard;

// Theme support
pub mod theme;

pub use vulkan::{Renderer, VulkanContext};

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

    /// Widget container for managing UI widgets
    widgets: widgets::container::WidgetContainer,

    /// Current window size (for layout calculations)
    width: u32,
    height: u32,
}

impl GuiContext {
    /// Create a new GUI context attached to a parent window (plugin mode)
    /// Attach the GUI to a parent window.
    ///
    /// # Safety
    ///
    /// The `parent` pointer must be a valid raw window handle for the target platform
    /// (NSView* on macOS, HWND on Windows) and must remain valid for the lifetime of the GUI.
    pub unsafe fn attach(parent: *mut std::ffi::c_void, width: u32, height: u32) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            use pal::NativeWindow;
            let window = pal::MacOSWindow::attach(parent)?;
            Ok(GuiContext {
                window,
                widgets: widgets::container::WidgetContainer::new(),
                width,
                height,
            })
        }

        #[cfg(target_os = "windows")]
        {
            use pal::NativeWindow;
            let window = pal::Win32Window::attach(parent)?;
            Ok(GuiContext {
                window,
                widgets: widgets::container::WidgetContainer::new(),
                width,
                height,
            })
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

    /// Get mutable access to the widget container
    ///
    /// Use this to add widgets, set layouts, etc.
    pub fn widgets_mut(&mut self) -> &mut widgets::container::WidgetContainer {
        &mut self.widgets
    }

    /// Get the widget container
    pub fn widgets(&self) -> &widgets::container::WidgetContainer {
        &self.widgets
    }

    /// Process a UI event
    ///
    /// This routes events to the widget container, which handles hit testing,
    /// focus management, and event dispatch to individual widgets.
    pub fn handle_event(&mut self, event: UIEvent) -> Result<Vec<widgets::EventResult>> {
        // Convert our UIEvent to pal::UIEvent for the widget container
        let pal_event = match event {
            UIEvent::MouseDown { x, y, button } => {
                let button_id = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                };
                pal::UIEvent::MouseDown {
                    x,
                    y,
                    button: button_id,
                }
            }
            UIEvent::MouseUp { x, y, button } => {
                let button_id = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                };
                pal::UIEvent::MouseUp {
                    x,
                    y,
                    button: button_id,
                }
            }
            UIEvent::MouseMove { x, y } => pal::UIEvent::MouseMove { x, y },
            UIEvent::KeyDown { .. } => {
                // TODO: Map Key enum to keycode properly
                pal::UIEvent::KeyDown { keycode: 0 }
            }
            UIEvent::KeyUp { .. } => {
                // TODO: Map Key enum to keycode properly
                pal::UIEvent::KeyUp { keycode: 0 }
            }
            UIEvent::Resize { width, height } => {
                self.width = width;
                self.height = height;
                // Apply layout when window resizes
                self.widgets.apply_layout(width as f32, height as f32);
                return Ok(Vec::new());
            }
        };

        Ok(self.widgets.handle_ui_event(pal_event))
    }

    /// Render the UI
    ///
    /// This should be called from the render loop to draw all widgets.
    /// Note: Currently a no-op until we integrate with the Renderer.
    pub fn render(&mut self) -> Result<()> {
        // TODO: Integrate with Renderer to actually draw widgets
        // For now, this is a placeholder
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

    /// Get mutable access to underlying window (for setting event callbacks)
    #[cfg(target_os = "macos")]
    pub fn get_window_mut(&mut self) -> Option<&mut pal::MacOSWindow> {
        Some(&mut self.window)
    }

    #[cfg(target_os = "windows")]
    pub fn get_window_mut(&mut self) -> Option<&mut pal::Win32Window> {
        Some(&mut self.window)
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
