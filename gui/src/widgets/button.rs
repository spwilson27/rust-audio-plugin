//! Button widget - clickable UI element

use super::{EventResult, Rect, Widget, WidgetEvent, WidgetId};
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

/// Button widget state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

/// Clickable button widget
pub struct Button {
    id: WidgetId,
    bounds: Rect,
    label: String,
    state: ButtonState,
    focused: bool,
    enabled: bool,
}

impl Button {
    /// Create a new button with the given label and position
    pub fn new(label: impl Into<String>, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            label: label.into(),
            state: ButtonState::Normal,
            focused: false,
            enabled: true,
        }
    }

    /// Set whether the button is enabled
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if the button is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the current button state
    pub fn state(&self) -> ButtonState {
        self.state
    }

    /// Get the button's label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Set the button's label
    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
    }
}

impl Widget for Button {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> WidgetId {
        self.id
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResult {
        if !self.enabled {
            return EventResult::NotHandled;
        }

        match event {
            WidgetEvent::MouseDown { .. } => {
                self.state = ButtonState::Pressed;
                EventResult::Handled
            }
            WidgetEvent::MouseUp { .. } => {
                // Only trigger click if we were in pressed state
                let was_pressed = self.state == ButtonState::Pressed;
                self.state = ButtonState::Hovered;

                if was_pressed {
                    // Button was clicked!
                    EventResult::ValueChanged(1.0)
                } else {
                    EventResult::Handled
                }
            }
            WidgetEvent::MouseEnter => {
                if self.state != ButtonState::Pressed {
                    self.state = ButtonState::Hovered;
                }
                EventResult::Handled
            }
            WidgetEvent::MouseExit => {
                self.state = ButtonState::Normal;
                EventResult::Handled
            }
            WidgetEvent::KeyDown { keycode } => {
                // Space or Enter activates button
                const SPACE_KEY: u32 = 49; // macOS keycode
                const ENTER_KEY: u32 = 36; // macOS keycode

                if *keycode == SPACE_KEY || *keycode == ENTER_KEY {
                    self.state = ButtonState::Pressed;
                    EventResult::Handled
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::KeyUp { keycode } => {
                const SPACE_KEY: u32 = 49;
                const ENTER_KEY: u32 = 36;

                if (*keycode == SPACE_KEY || *keycode == ENTER_KEY)
                    && self.state == ButtonState::Pressed
                {
                    self.state = if self.focused {
                        ButtonState::Hovered
                    } else {
                        ButtonState::Normal
                    };
                    // Keyboard activation
                    EventResult::ValueChanged(1.0)
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::FocusGained => EventResult::Handled,
            WidgetEvent::FocusLost => {
                self.state = ButtonState::Normal;
                EventResult::Handled
            }
            _ => EventResult::NotHandled,
        }
    }

    fn render(
        &self,
        shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        _vulkan_context: &crate::VulkanContext,
        _font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        use crate::theme::colors::*;

        // Determine colors based on state
        let (bg_color, border_color, text_color) = if !self.enabled {
            (BACKGROUND_ROOT, BORDER, TEXT_DISABLED)
        } else {
            match self.state {
                ButtonState::Normal => (BACKGROUND_WIDGET, BORDER, TEXT_PRIMARY),
                ButtonState::Hovered => (BACKGROUND_HOVER, BORDER_HOVER, TEXT_PRIMARY),
                ButtonState::Pressed => (BACKGROUND_PRESSED, BORDER_FOCUS, TEXT_PRIMARY),
            }
        };

        // Draw button background with rounded corners
        shape_renderer.draw_rect(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            bg_color,
            4.0, // border radius
        );

        // Draw border (as a slightly larger rect underneath, or outline)
        // Draw outline:
        shape_renderer.draw_rect(
            self.bounds.x - 1.0,
            self.bounds.y - 1.0,
            self.bounds.width + 2.0,
            self.bounds.height + 2.0,
            border_color,
            4.0,
        );

        // Draw focus indicator if focused
        if self.focused {
            shape_renderer.draw_rect(
                self.bounds.x - 2.0,
                self.bounds.y - 2.0,
                self.bounds.width + 4.0,
                self.bounds.height + 4.0,
                BORDER_FOCUS, // Focus ring
                5.0,
            );
        }

        // Draw label text (centered)
        let text_size = 16.0;
        let text_width = _font_atlas.measure_text(&self.label, text_size);

        let text_x = self.bounds.x + (self.bounds.width - text_width) / 2.0;
        let text_y = self.bounds.y + self.bounds.height / 2.0 + 6.0; // Vertical center (approximate)

        let _ = text_renderer.draw_text(
            _vulkan_context,
            _font_atlas,
            &self.label,
            text_x,
            text_y,
            text_size,
            text_color,
        );
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.bounds.x = x;
        self.bounds.y = y;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn can_focus(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_creation() {
        let button = Button::new("Click Me", 10.0, 20.0, 100.0, 30.0);
        assert_eq!(button.label(), "Click Me");
        assert_eq!(button.state(), ButtonState::Normal);
        assert!(button.is_enabled());
        assert!(!button.is_focused());
    }

    #[test]
    fn test_button_click() {
        let mut button = Button::new("Test", 0.0, 0.0, 100.0, 30.0);

        // Mouse down
        let result = button.handle_event(&WidgetEvent::MouseDown {
            x: 50.0,
            y: 15.0,
            button: 0,
        });
        assert_eq!(result, EventResult::Handled);
        assert_eq!(button.state(), ButtonState::Pressed);

        // Mouse up (click!)
        let result = button.handle_event(&WidgetEvent::MouseUp {
            x: 50.0,
            y: 15.0,
            button: 0,
        });
        assert_eq!(result, EventResult::ValueChanged(1.0));
        assert_eq!(button.state(), ButtonState::Hovered);
    }

    #[test]
    fn test_button_hover() {
        let mut button = Button::new("Test", 0.0, 0.0, 100.0, 30.0);

        button.handle_event(&WidgetEvent::MouseEnter);
        assert_eq!(button.state(), ButtonState::Hovered);

        button.handle_event(&WidgetEvent::MouseExit);
        assert_eq!(button.state(), ButtonState::Normal);
    }

    #[test]
    fn test_button_keyboard_activation() {
        let mut button = Button::new("Test", 0.0, 0.0, 100.0, 30.0);
        button.set_focused(true);

        // Press space
        let result = button.handle_event(&WidgetEvent::KeyDown { keycode: 49 });
        assert_eq!(result, EventResult::Handled);
        assert_eq!(button.state(), ButtonState::Pressed);

        // Release space
        let result = button.handle_event(&WidgetEvent::KeyUp { keycode: 49 });
        assert_eq!(result, EventResult::ValueChanged(1.0));
    }

    #[test]
    fn test_disabled_button() {
        let mut button = Button::new("Test", 0.0, 0.0, 100.0, 30.0);
        button.set_enabled(false);

        // Should not respond to clicks
        let result = button.handle_event(&WidgetEvent::MouseDown {
            x: 50.0,
            y: 15.0,
            button: 0,
        });
        assert_eq!(result, EventResult::NotHandled);
        assert_eq!(button.state(), ButtonState::Normal);

        // Cannot be focused
        assert!(!button.can_focus());
    }

    #[test]
    fn test_button_id_uniqueness() {
        let button1 = Button::new("Button 1", 0.0, 0.0, 100.0, 30.0);
        let button2 = Button::new("Button 2", 0.0, 0.0, 100.0, 30.0);
        assert_ne!(button1.id(), button2.id());
    }
}
