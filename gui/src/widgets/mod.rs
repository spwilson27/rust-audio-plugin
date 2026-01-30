//! Widget framework for building interactive UIs
//!
//! This module provides a flexible widget system for creating interactive UIs
//! with support for:
//! - Event handling (mouse, keyboard, focus)
//! - Rendering via Vulkan
//! - Layout management
//! - Automated testing via RPC event
pub mod button;
pub mod container;
pub mod dropdown;
pub mod knob;
pub mod label;
pub mod layout;
pub mod selector;
pub mod slider;
pub mod textbox;
pub mod widget_id;

pub use button::Button;
pub use dropdown::Dropdown;
pub use knob::Knob;
pub use label::Label;
pub use selector::Selector;
pub use slider::Slider;
pub use textbox::Textbox;
pub use widget_id::WidgetId;

use super::vulkan::shape_renderer::ShapeRenderer;
use super::vulkan::text_renderer::TextRenderer;

/// Core trait for all UI widgets
pub trait Widget: std::any::Any + Send {
    /// Get reference as Any for downcasting
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get moving reference as Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Get the unique ID of this widget
    fn id(&self) -> WidgetId;

    /// Handle a widget event
    /// Returns whether the event was handled and any resulting value change
    fn handle_event(&mut self, event: &WidgetEvent) -> EventResult;

    /// Render the widget using the provided renderers
    fn render(
        &self,
        shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        vulkan_context: &super::VulkanContext,
        font_atlas: &mut super::vulkan::text_renderer::FontAtlas,
        screen_width: u32,
        screen_height: u32,
    );

    /// Render overlay elements (popups, tooltips) that should appear above all other widgets
    fn render_overlay(
        &self,
        _shape_renderer: &mut ShapeRenderer,
        _text_renderer: &mut TextRenderer,
        _vulkan_context: &super::VulkanContext,
        _font_atlas: &mut super::vulkan::text_renderer::FontAtlas,
        _screen_width: u32,
        _screen_height: u32,
    ) {
    }

    /// Get the bounding rectangle of the overlay, if any
    /// This is used for hit testing to interpret clicks on the overlay
    fn overlay_bounds(&self) -> Option<Rect> {
        None
    }

    /// Get the bounding rectangle of this widget
    fn bounds(&self) -> Rect;

    /// Set the position of this widget (top-left corner)
    fn set_position(&mut self, x: f32, y: f32);

    /// Set the size of this widget
    fn set_size(&mut self, width: f32, height: f32);

    /// Check if this widget currently has focus
    fn is_focused(&self) -> bool;

    /// Set focus state for this widget
    fn set_focused(&mut self, focused: bool);

    /// Check if this widget can receive keyboard focus
    fn can_focus(&self) -> bool {
        true
    }
}

/// Widget-specific events (higher level than raw UIEvent)
#[derive(Debug, Clone)]
pub enum WidgetEvent {
    /// Mouse button pressed within widget bounds
    MouseDown { x: f64, y: f64, button: u32 },

    /// Mouse button released within widget bounds
    MouseUp { x: f64, y: f64, button: u32 },

    /// Mouse moved within widget bounds
    MouseMove { x: f64, y: f64 },

    /// Mouse entered widget bounds
    MouseEnter,

    /// Mouse exited widget bounds
    MouseExit,

    /// Key pressed while widget has focus
    KeyDown {
        keycode: u32,
        modifiers: pal::Modifiers,
    },

    /// Key released while widget has focus
    KeyUp {
        keycode: u32,
        modifiers: pal::Modifiers,
    },
    /// Text input event (UTF-8 string)
    TextInput(String),

    /// Widget gained focus
    FocusGained,

    /// Widget lost focus
    FocusLost,
}

/// Result of handling an event
#[derive(Debug, Clone, PartialEq)]
pub enum EventResult {
    /// Event was handled by this widget
    Handled,

    /// Event was not relevant to this widget
    NotHandled,

    /// Event was handled and the widget's value changed
    /// The f64 is the new normalized value [0.0, 1.0]
    ValueChanged(f64),

    /// Widget requests mouse capture (all mouse events go to this widget until release)
    CaptureMouse,

    /// Widget releases mouse capture
    ReleaseMouse,
}

/// Rectangular bounds in screen space (pixels)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let x = x as f32;
        let y = y as f32;
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    /// Get the center point of this rectangle
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);

        // Inside
        assert!(rect.contains(50.0, 40.0));
        assert!(rect.contains(10.0, 20.0)); // Top-left corner
        assert!(rect.contains(110.0, 70.0)); // Bottom-right corner

        // Outside
        assert!(!rect.contains(5.0, 40.0)); // Left
        assert!(!rect.contains(120.0, 40.0)); // Right
        assert!(!rect.contains(50.0, 15.0)); // Above
        assert!(!rect.contains(50.0, 75.0)); // Below
    }

    #[test]
    fn test_rect_center() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let (cx, cy) = rect.center();
        assert_eq!(cx, 60.0);
        assert_eq!(cy, 45.0);
    }
}
