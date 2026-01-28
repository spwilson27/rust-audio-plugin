//! Slider widget - adjustable value control

use super::{EventResult, Rect, Widget, WidgetEvent, WidgetId};
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

/// Slider orientation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

/// Slider widget for adjusting values
pub struct Slider {
    id: WidgetId,
    bounds: Rect,
    orientation: SliderOrientation,
    value: f64, // Normalized [0.0, 1.0]
    focused: bool,
    dragging: bool,
    hovered: bool,
}

impl Slider {
    /// Create a new horizontal slider
    pub fn new_horizontal(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            orientation: SliderOrientation::Horizontal,
            value: 0.5,
            focused: false,
            dragging: false,
            hovered: false,
        }
    }

    /// Create a new vertical slider
    pub fn new_vertical(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            orientation: SliderOrientation::Vertical,
            value: 0.5,
            focused: false,
            dragging: false,
            hovered: false,
        }
    }

    /// Get the current value [0.0, 1.0]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Set the value [0.0, 1.0]
    pub fn set_value(&mut self, value: f64) {
        self.value = value.clamp(0.0, 1.0);
    }

    /// Get the orientation
    pub fn orientation(&self) -> SliderOrientation {
        self.orientation
    }

    /// Update value based on mouse position
    fn update_value_from_position(&mut self, x: f64, y: f64) -> bool {
        let old_value = self.value;

        match self.orientation {
            SliderOrientation::Horizontal => {
                let relative_x = (x as f32 - self.bounds.x) / self.bounds.width;
                self.value = relative_x.clamp(0.0, 1.0) as f64;
            }
            SliderOrientation::Vertical => {
                // Inverted: top = 1.0, bottom = 0.0
                let relative_y = (y as f32 - self.bounds.y) / self.bounds.height;
                self.value = (1.0 - relative_y).clamp(0.0, 1.0) as f64;
            }
        }

        // Return true if value changed
        (self.value - old_value).abs() > f64::EPSILON
    }
}

impl Widget for Slider {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResult {
        match event {
            WidgetEvent::MouseDown { x, y, .. } => {
                self.dragging = true;
                if self.update_value_from_position(*x, *y) {
                    EventResult::ValueChanged(self.value)
                } else {
                    EventResult::Handled
                }
            }
            WidgetEvent::MouseUp { .. } => {
                self.dragging = false;
                EventResult::Handled
            }
            WidgetEvent::MouseMove { x, y } => {
                if self.dragging {
                    if self.update_value_from_position(*x, *y) {
                        EventResult::ValueChanged(self.value)
                    } else {
                        EventResult::Handled
                    }
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::MouseEnter => {
                self.hovered = true;
                EventResult::Handled
            }
            WidgetEvent::MouseExit => {
                self.hovered = false;
                EventResult::Handled
            }
            WidgetEvent::KeyDown { keycode } => {
                // Arrow keys for fine adjustment
                const LEFT_ARROW: u32 = 123; // macOS
                const RIGHT_ARROW: u32 = 124;
                const DOWN_ARROW: u32 = 125;
                const UP_ARROW: u32 = 126;

                let old_value = self.value;
                let step = 0.01; // 1% step

                match self.orientation {
                    SliderOrientation::Horizontal => {
                        if *keycode == LEFT_ARROW {
                            self.value = (self.value - step).clamp(0.0, 1.0);
                        } else if *keycode == RIGHT_ARROW {
                            self.value = (self.value + step).clamp(0.0, 1.0);
                        }
                    }
                    SliderOrientation::Vertical => {
                        if *keycode == DOWN_ARROW {
                            self.value = (self.value - step).clamp(0.0, 1.0);
                        } else if *keycode == UP_ARROW {
                            self.value = (self.value + step).clamp(0.0, 1.0);
                        }
                    }
                }

                if (self.value - old_value).abs() > f64::EPSILON {
                    EventResult::ValueChanged(self.value)
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::FocusGained | WidgetEvent::FocusLost => EventResult::Handled,
            _ => EventResult::NotHandled,
        }
    }

    fn render(
        &self,
        shape_renderer: &mut ShapeRenderer,
        _text_renderer: &mut TextRenderer,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        // Colors
        let track_color = [0.2, 0.2, 0.2, 1.0];
        let fill_color = [0.3, 0.5, 0.7, 1.0]; // Blue
        let thumb_color = if self.hovered || self.dragging {
            [0.5, 0.7, 0.9, 1.0] // Lighter blue
        } else {
            [0.4, 0.6, 0.8, 1.0]
        };
        let focus_color = [0.5, 0.7, 1.0, 0.5];

        match self.orientation {
            SliderOrientation::Horizontal => {
                // Draw track
                shape_renderer.draw_rect(
                    self.bounds.x,
                    self.bounds.y + self.bounds.height / 2.0 - 2.0,
                    self.bounds.width,
                    4.0,
                    track_color,
                    2.0,
                );

                // Draw filled portion
                let fill_width = self.bounds.width * self.value as f32;
                shape_renderer.draw_rect(
                    self.bounds.x,
                    self.bounds.y + self.bounds.height / 2.0 - 2.0,
                    fill_width,
                    4.0,
                    fill_color,
                    2.0,
                );

                // Draw thumb
                let thumb_x = self.bounds.x + fill_width - 6.0;
                let thumb_y = self.bounds.y + self.bounds.height / 2.0 - 8.0;
                shape_renderer.draw_rect(thumb_x, thumb_y, 12.0, 16.0, thumb_color, 4.0);

                // Draw focus indicator
                if self.focused {
                    shape_renderer.draw_rect(
                        self.bounds.x - 2.0,
                        self.bounds.y - 2.0,
                        self.bounds.width + 4.0,
                        self.bounds.height + 4.0,
                        focus_color,
                        3.0,
                    );
                }
            }
            SliderOrientation::Vertical => {
                // Draw track
                shape_renderer.draw_rect(
                    self.bounds.x + self.bounds.width / 2.0 - 2.0,
                    self.bounds.y,
                    4.0,
                    self.bounds.height,
                    track_color,
                    2.0,
                );

                // Draw filled portion (from bottom)
                let fill_height = self.bounds.height * self.value as f32;
                let fill_y = self.bounds.y + self.bounds.height - fill_height;
                shape_renderer.draw_rect(
                    self.bounds.x + self.bounds.width / 2.0 - 2.0,
                    fill_y,
                    4.0,
                    fill_height,
                    fill_color,
                    2.0,
                );

                // Draw thumb
                let thumb_x = self.bounds.x + self.bounds.width / 2.0 - 8.0;
                let thumb_y = fill_y - 6.0;
                shape_renderer.draw_rect(thumb_x, thumb_y, 16.0, 12.0, thumb_color, 4.0);

                // Draw focus indicator
                if self.focused {
                    shape_renderer.draw_rect(
                        self.bounds.x - 2.0,
                        self.bounds.y - 2.0,
                        self.bounds.width + 4.0,
                        self.bounds.height + 4.0,
                        focus_color,
                        3.0,
                    );
                }
            }
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_creation() {
        let slider = Slider::new_horizontal(10.0, 20.0, 200.0, 30.0);
        assert_eq!(slider.value(), 0.5);
        assert_eq!(slider.orientation(), SliderOrientation::Horizontal);
        assert!(!slider.is_focused());
    }

    #[test]
    fn test_horizontal_slider_drag() {
        let mut slider = Slider::new_horizontal(0.0, 0.0, 100.0, 30.0);

        // Click at 75% position
        let result = slider.handle_event(&WidgetEvent::MouseDown {
            x: 75.0,
            y: 15.0,
            button: 0,
        });

        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 0.75).abs() < 0.01);
                assert_eq!(slider.value(), val);
            }
            _ => panic!("Expected ValueChanged"),
        }

        // Drag to 25%
        let result = slider.handle_event(&WidgetEvent::MouseMove { x: 25.0, y: 15.0 });

        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 0.25).abs() < 0.01);
            }
            _ => panic!("Expected ValueChanged"),
        }
    }

    #[test]
    fn test_vertical_slider_drag() {
        let mut slider = Slider::new_vertical(0.0, 0.0, 30.0, 100.0);

        // Click at top (which should be value 1.0)
        let result = slider.handle_event(&WidgetEvent::MouseDown {
            x: 15.0,
            y: 0.0,
            button: 0,
        });

        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 1.0).abs() < 0.01);
            }
            _ => panic!("Expected ValueChanged"),
        }

        // Click at bottom (which should be value 0.0)
        let result = slider.handle_event(&WidgetEvent::MouseDown {
            x: 15.0,
            y: 100.0,
            button: 0,
        });

        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 0.0).abs() < 0.01);
            }
            _ => panic!("Expected ValueChanged"),
        }
    }

    #[test]
    fn test_slider_keyboard_adjustment() {
        let mut slider = Slider::new_horizontal(0.0, 0.0, 100.0, 30.0);
        slider.set_value(0.5);

        // Right arrow should increase
        let result = slider.handle_event(&WidgetEvent::KeyDown { keycode: 124 });
        match result {
            EventResult::ValueChanged(val) => {
                assert!(val > 0.5);
                assert!((val - 0.51).abs() < 0.001);
            }
            _ => panic!("Expected ValueChanged"),
        }

        // Left arrow should decrease
        let result = slider.handle_event(&WidgetEvent::KeyDown { keycode: 123 });
        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected ValueChanged"),
        }
    }

    #[test]
    fn test_slider_clamping() {
        let mut slider = Slider::new_horizontal(0.0, 0.0, 100.0, 30.0);

        // Click beyond right edge
        slider.handle_event(&WidgetEvent::MouseDown {
            x: 150.0,
            y: 15.0,
            button: 0,
        });
        assert_eq!(slider.value(), 1.0);

        // Click before left edge
        slider.handle_event(&WidgetEvent::MouseDown {
            x: -10.0,
            y: 15.0,
            button: 0,
        });
        assert_eq!(slider.value(), 0.0);
    }

    #[test]
    fn test_slider_set_value() {
        let mut slider = Slider::new_horizontal(0.0, 0.0, 100.0, 30.0);
        slider.set_value(0.75);
        assert_eq!(slider.value(), 0.75);

        // Test clamping
        slider.set_value(1.5);
        assert_eq!(slider.value(), 1.0);

        slider.set_value(-0.5);
        assert_eq!(slider.value(), 0.0);
    }

    #[test]
    fn test_slider_id_uniqueness() {
        let slider1 = Slider::new_horizontal(0.0, 0.0, 100.0, 30.0);
        let slider2 = Slider::new_horizontal(0.0, 0.0, 100.0, 30.0);
        assert_ne!(slider1.id(), slider2.id());
    }
}
