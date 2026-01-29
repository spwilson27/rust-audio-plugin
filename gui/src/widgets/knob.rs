//! Knob widget - rotary control for audio parameters

use super::{EventResult, Rect, Widget, WidgetEvent, WidgetId};
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

/// Knob widget for rotary parameter control
///
/// Knobs are common in audio plugins for controlling parameters like volume,
/// frequency, resonance, etc. This widget uses vertical drag interaction:
/// - Drag up to increase value
/// - Drag down to decrease value
pub struct Knob {
    id: WidgetId,
    bounds: Rect,
    value: f64, // Normalized [0.0, 1.0]
    focused: bool,
    dragging: bool,
    drag_start_y: f64,
    drag_start_value: f64,
    hovered: bool,

    // Angle range configuration (degrees)
    min_angle: f32, // Default: 225° (bottom-left)
    max_angle: f32, // Default: -45° (bottom-right) = 315°

    // Drag sensitivity (pixels per full range)
    sensitivity: f64, // Default: 100 pixels = 0.0 to 1.0
}

impl Knob {
    /// Create a new knob widget
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        let size = radius * 2.0;
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x - radius, y - radius, size, size),
            value: 0.5,
            focused: false,
            dragging: false,
            drag_start_y: 0.0,
            drag_start_value: 0.0,
            hovered: false,
            min_angle: 225.0,   // Bottom-left
            max_angle: -45.0,   // Bottom-right (= 315°)
            sensitivity: 100.0, // 100 pixels for full range
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

    /// Set the angle range in degrees
    ///
    /// # Arguments
    /// * `min_angle` - Angle at value 0.0 (degrees, 0° = right, 90° = down)
    /// * `max_angle` - Angle at value 1.0 (degrees)
    pub fn set_angle_range(&mut self, min_angle: f32, max_angle: f32) {
        self.min_angle = min_angle;
        self.max_angle = max_angle;
    }

    /// Set drag sensitivity (pixels required to go from 0.0 to 1.0)
    pub fn set_sensitivity(&mut self, sensitivity: f64) {
        self.sensitivity = sensitivity.max(10.0); // Minimum 10 pixels
    }

    /// Get the current angle in degrees based on value
    fn value_to_angle(&self) -> f32 {
        let range = self.max_angle - self.min_angle;
        self.min_angle + range * self.value as f32
    }

    /// Get the radius of the knob
    fn radius(&self) -> f32 {
        self.bounds.width / 2.0
    }

    /// Get the center point of the knob
    fn center(&self) -> (f32, f32) {
        self.bounds.center()
    }
}

impl Widget for Knob {
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
        match event {
            WidgetEvent::MouseDown { x: _, y, .. } => {
                self.dragging = true;
                self.drag_start_y = *y;
                self.drag_start_value = self.value;
                EventResult::CaptureMouse
            }
            WidgetEvent::MouseUp { .. } => {
                if self.dragging {
                    self.dragging = false;
                    EventResult::Handled
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::MouseMove { y, .. } => {
                if self.dragging {
                    // Vertical drag: up = increase, down = decrease
                    let delta_y = self.drag_start_y - y; // Inverted (up = positive)
                    let delta_value = delta_y / self.sensitivity;
                    let new_value = (self.drag_start_value + delta_value).clamp(0.0, 1.0);

                    if (new_value - self.value).abs() > f64::EPSILON {
                        self.value = new_value;
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
            WidgetEvent::KeyDown {
                keycode,
                modifiers: _,
            } => {
                // Arrow keys for fine adjustment
                const UP_ARROW: u32 = 126; // macOS
                const DOWN_ARROW: u32 = 125;

                let old_value = self.value;
                let step = 0.01; // 1% step

                if *keycode == UP_ARROW {
                    self.value = (self.value + step).clamp(0.0, 1.0);
                } else if *keycode == DOWN_ARROW {
                    self.value = (self.value - step).clamp(0.0, 1.0);
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
        _vulkan_context: &crate::VulkanContext,
        _font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        use crate::theme::colors::*;

        let (cx, cy) = self.center();
        let radius = self.radius();

        // Colors
        let background_color = BACKGROUND_WIDGET;
        let track_color = BACKGROUND_PRESSED; // Inner track
        let value_color = if self.hovered || self.dragging {
            ACCENT_HOVER
        } else {
            ACCENT
        };
        let indicator_color = TEXT_PRIMARY; // White line

        // Draw background circle
        shape_renderer.draw_circle(cx, cy, radius, background_color);

        // Draw track circle (slightly smaller)
        let track_radius = radius * 0.85;
        shape_renderer.draw_circle(cx, cy, track_radius, track_color);

        // TODO: Draw arc for the value range

        // Draw value indicator line (from center to edge)
        let angle_rad = self.value_to_angle().to_radians();
        let indicator_end = radius * 0.8;

        let x2 = cx + angle_rad.cos() * indicator_end;
        let y2 = cy + angle_rad.sin() * indicator_end;

        // Draw indicator as a thin rectangle (approximating a line)
        let line_width = 2.0;

        // Approximation: draw a small rect at the indicator position
        shape_renderer.draw_rect(
            x2 - line_width / 2.0,
            y2 - line_width / 2.0,
            line_width,
            line_width * 3.0, // Make it longer? No, this is just a dot/rect.
            // Previous code: line_width * 3.0.
            // Let's stick to previous geometry but use theme color.
            indicator_color,
            1.0,
        );

        // Draw a dot at the current value position on the edge
        let dot_radius = 4.0;
        let dot_x = cx + angle_rad.cos() * track_radius;
        let dot_y = cy + angle_rad.sin() * track_radius;
        shape_renderer.draw_circle(dot_x, dot_y, dot_radius, value_color);

        // Draw focus indicator
        if self.focused {
            shape_renderer.draw_circle(cx, cy, radius + 3.0, BORDER_FOCUS);
        }

        // Draw value text below knob
        let value_text = format!("{:.0}%", self.value * 100.0);
        let text_size = 14.0;
        let text_width = _font_atlas.measure_text(&value_text, text_size);

        let text_x = cx - text_width / 2.0; // Centered
        let text_y = cy + radius + 20.0; // Below the knob

        let _ = _text_renderer.draw_text(
            _vulkan_context,
            _font_atlas,
            &value_text,
            text_x,
            text_y,
            text_size,
            TEXT_PRIMARY,
        );
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_position(&mut self, x: f32, y: f32) {
        let radius = self.radius();
        self.bounds.x = x - radius;
        self.bounds.y = y - radius;
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
    fn test_knob_creation() {
        let knob = Knob::new(50.0, 50.0, 20.0);
        assert_eq!(knob.value(), 0.5);
        assert!(!knob.is_focused());
        assert_eq!(knob.radius(), 20.0);
    }

    #[test]
    fn test_knob_vertical_drag() {
        let mut knob = Knob::new(50.0, 50.0, 20.0);
        knob.set_sensitivity(100.0);
        knob.set_value(0.5);

        // Start drag
        knob.handle_event(&WidgetEvent::MouseDown {
            x: 50.0,
            y: 50.0,
            button: 0,
        });

        // Drag up 50 pixels (should increase value by 0.5)
        let result = knob.handle_event(&WidgetEvent::MouseMove {
            x: 50.0,
            y: 0.0, // 50 pixels up
        });

        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 1.0).abs() < 0.01); // Should be at max
                assert_eq!(knob.value(), 1.0);
            }
            _ => panic!("Expected ValueChanged"),
        }

        // Drag down 100 pixels from start (should go to 0.0)
        let result = knob.handle_event(&WidgetEvent::MouseMove {
            x: 50.0,
            y: 150.0, // 100 pixels down from start
        });

        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 0.0).abs() < 0.01);
                assert_eq!(knob.value(), 0.0);
            }
            _ => panic!("Expected ValueChanged"),
        }
    }

    #[test]
    fn test_knob_keyboard_adjustment() {
        let mut knob = Knob::new(50.0, 50.0, 20.0);
        knob.set_value(0.5);

        // Up arrow should increase
        let result = knob.handle_event(&WidgetEvent::KeyDown {
            keycode: 126,
            modifiers: pal::Modifiers::empty(),
        });
        match result {
            EventResult::ValueChanged(val) => {
                assert!(val > 0.5);
                assert!((val - 0.51).abs() < 0.001);
            }
            _ => panic!("Expected ValueChanged"),
        }

        // Down arrow should decrease
        let result = knob.handle_event(&WidgetEvent::KeyDown {
            keycode: 125,
            modifiers: pal::Modifiers::empty(),
        });
        match result {
            EventResult::ValueChanged(val) => {
                assert!((val - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected ValueChanged"),
        }
    }

    #[test]
    fn test_knob_value_clamping() {
        let mut knob = Knob::new(50.0, 50.0, 20.0);

        knob.set_value(1.5);
        assert_eq!(knob.value(), 1.0);

        knob.set_value(-0.5);
        assert_eq!(knob.value(), 0.0);
    }

    #[test]
    fn test_knob_angle_calculation() {
        let mut knob = Knob::new(50.0, 50.0, 20.0);

        knob.set_value(0.0);
        assert!((knob.value_to_angle() - 225.0).abs() < 0.1);

        knob.set_value(1.0);
        assert!((knob.value_to_angle() - (-45.0)).abs() < 0.1);

        knob.set_value(0.5);
        // Mid-point should be 90° (270° - 180°/2 = 90°)
        let expected = 225.0 + ((-45.0) - 225.0) * 0.5;
        assert!((knob.value_to_angle() - expected).abs() < 0.1);
    }

    #[test]
    fn test_knob_sensitivity() {
        let mut knob = Knob::new(50.0, 50.0, 20.0);
        knob.set_sensitivity(50.0); // 50 pixels for full range
        knob.set_value(0.5);

        knob.handle_event(&WidgetEvent::MouseDown {
            x: 50.0,
            y: 50.0,
            button: 0,
        });

        // Drag up 25 pixels (should increase by 0.5 with sensitivity=50)
        knob.handle_event(&WidgetEvent::MouseMove { x: 50.0, y: 25.0 });
        assert_eq!(knob.value(), 1.0);
    }

    #[test]
    fn test_knob_id_uniqueness() {
        let knob1 = Knob::new(50.0, 50.0, 20.0);
        let knob2 = Knob::new(50.0, 50.0, 20.0);
        assert_ne!(knob1.id(), knob2.id());
    }
}
