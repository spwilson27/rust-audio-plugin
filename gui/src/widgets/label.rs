use super::{EventResult, Rect, Widget, WidgetEvent, WidgetId};
use crate::theme::colors::TEXT_PRIMARY;
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

/// Static text label widget
pub struct Label {
    id: WidgetId,
    bounds: Rect,
    text: String,
    color: [f32; 4],
    centered: bool,
}

impl Label {
    /// Create a new label
    pub fn new(text: impl Into<String>, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            text: text.into(),
            color: TEXT_PRIMARY,
            centered: false,
        }
    }

    /// Set text color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Set centered alignment
    pub fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }
}

impl Widget for Label {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn id(&self) -> WidgetId {
        self.id
    }

    fn handle_event(&mut self, _event: &WidgetEvent) -> EventResult {
        // Labels don't handle events by default
        EventResult::NotHandled
    }

    fn render(
        &self,
        _shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        vulkan_context: &crate::VulkanContext,
        font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        let text_size = 16.0;

        let (x, y) = if self.centered {
            let text_width = font_atlas.measure_text(&self.text, text_size);
            let x = self.bounds.x + (self.bounds.width - text_width) / 2.0;
            let y = self.bounds.y + self.bounds.height / 2.0 + 6.0; // Approx vertical center
            (x, y)
        } else {
            // Default vert alignment only
            (
                self.bounds.x,
                self.bounds.y + self.bounds.height / 2.0 + 6.0,
            )
        };

        let _ = text_renderer.draw_text(
            vulkan_context,
            font_atlas,
            &self.text,
            x,
            y,
            text_size,
            self.color,
        );
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.bounds.x = x;
        self.bounds.y = y;
    }

    fn set_size(&mut self, width: f32, height: f32) {
        self.bounds.width = width;
        self.bounds.height = height;
    }

    fn is_focused(&self) -> bool {
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn can_focus(&self) -> bool {
        false
    }
}
