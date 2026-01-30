use super::{Button, EventResult, Label, Rect, Widget, WidgetEvent, WidgetId};
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

/// A selector widget that cycles through a list of items using prev/next buttons
pub struct Selector {
    id: WidgetId,
    bounds: Rect,
    items: Vec<String>,
    selected_index: usize,

    // Child widgets
    prev_btn: Button,
    next_btn: Button,
    label: Label,
}

impl Selector {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        let btn_width = 30.0;
        let label_width = width - (btn_width * 2.0) - 10.0; // 5px padding each side

        let prev_btn = Button::new("<", x, y, btn_width, height);
        let next_btn = Button::new(">", x + width - btn_width, y, btn_width, height);
        let label = Label::new("", x + btn_width + 5.0, y, label_width, height).centered(true);

        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            items: Vec::new(),
            selected_index: 0,
            prev_btn,
            next_btn,
            label,
        }
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        if self.selected_index >= self.items.len() {
            self.selected_index = 0;
        }
        self.update_label();
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = index;
            self.update_label();
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected_index).map(|s| s.as_str())
    }

    fn update_label(&mut self) {
        if let Some(text) = self.items.get(self.selected_index) {
            self.label.set_text(text);
        } else {
            self.label.set_text("");
        }
    }

    fn layout_children(&mut self) {
        let x = self.bounds.x;
        let y = self.bounds.y;
        let w = self.bounds.width;
        let h = self.bounds.height;

        let btn_width = 30.0;
        let label_width = w - (btn_width * 2.0) - 10.0;

        self.prev_btn.set_position(x, y);
        self.prev_btn.set_size(btn_width, h);

        self.label.set_position(x + btn_width + 5.0, y);
        self.label.set_size(label_width, h);

        self.next_btn.set_position(x + w - btn_width, y);
        self.next_btn.set_size(btn_width, h);
    }
}

impl Widget for Selector {
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
        // Forward events to children
        // We need to check bounds or just let them handle key events if focused?
        // Mouse events need to be checked against bounds.
        // But since we are inside `handle_event` of Selector, `container` probably already checked Selector bounds?
        // Actually `container` dispatch logic iterates widgets.
        // If we implement `handle_event`, we are responsible for dispatching to interactive children.

        // Handle Prev Button
        match self.prev_btn.handle_event(event) {
            EventResult::ValueChanged(_) => {
                // Prev clicked
                if !self.items.is_empty() {
                    if self.selected_index > 0 {
                        self.selected_index -= 1;
                    } else {
                        self.selected_index = self.items.len() - 1;
                    }
                    self.update_label();
                    return EventResult::ValueChanged(self.selected_index as f64);
                }
                return EventResult::Handled;
            }
            EventResult::Handled => return EventResult::Handled,
            _ => {}
        }

        // Handle Next Button
        match self.next_btn.handle_event(event) {
            EventResult::ValueChanged(_) => {
                // Next clicked
                if !self.items.is_empty() {
                    if self.selected_index < self.items.len() - 1 {
                        self.selected_index += 1;
                    } else {
                        self.selected_index = 0;
                    }
                    self.update_label();
                    return EventResult::ValueChanged(self.selected_index as f64);
                }
                return EventResult::Handled;
            }
            EventResult::Handled => return EventResult::Handled,
            _ => {}
        }

        EventResult::NotHandled
    }

    fn render(
        &self,
        shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        vulkan_context: &crate::VulkanContext,
        font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        screen_width: u32,
        screen_height: u32,
    ) {
        // Render children
        self.prev_btn.render(
            shape_renderer,
            text_renderer,
            vulkan_context,
            font_atlas,
            screen_width,
            screen_height,
        );
        self.label.render(
            shape_renderer,
            text_renderer,
            vulkan_context,
            font_atlas,
            screen_width,
            screen_height,
        );
        self.next_btn.render(
            shape_renderer,
            text_renderer,
            vulkan_context,
            font_atlas,
            screen_width,
            screen_height,
        );
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_position(&mut self, x: f32, y: f32) {
        self.bounds.x = x;
        self.bounds.y = y;
        self.layout_children();
    }

    fn set_size(&mut self, width: f32, height: f32) {
        self.bounds.width = width;
        self.bounds.height = height;
        self.layout_children();
    }

    fn is_focused(&self) -> bool {
        // Selector itself doesn't focus, buttons do?
        // Or selector manages focus between buttons?
        false
    }

    fn set_focused(&mut self, _focused: bool) {}

    fn can_focus(&self) -> bool {
        false
    }
}
