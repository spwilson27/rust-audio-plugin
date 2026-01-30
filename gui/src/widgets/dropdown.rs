use super::{EventResult, Rect, Widget, WidgetEvent, WidgetId};
use crate::theme::colors::*;
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

pub struct Dropdown {
    id: WidgetId,
    bounds: Rect,
    items: Vec<String>,
    selected_index: usize,
    expanded: bool,
    hovered_item_index: Option<usize>,
    focused: bool,
    enabled: bool,
}

impl Dropdown {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            items: Vec::new(),
            selected_index: 0,
            expanded: false,
            hovered_item_index: None,
            focused: false,
            enabled: true,
        }
    }

    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
        if self.selected_index >= self.items.len() {
            self.selected_index = 0;
        }
    }

    pub fn set_selected_index(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected_index = index;
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected_index).map(|s| s.as_str())
    }

    fn item_height(&self) -> f32 {
        30.0
    }
}

impl Widget for Dropdown {
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
            WidgetEvent::MouseDown { x, y, .. } => {
                let x = *x as f32;
                let y = *y as f32;
                // If we are here, hit_test succeeded (either on header or overlay).
                // Just toggle or select based on where we clicked.

                if self.expanded {
                    // Check if clicked inside list (overlay)
                    let item_h = self.item_height();
                    let list_height = self.items.len() as f32 * item_h;
                    let list_y = self.bounds.y + self.bounds.height;

                    if y >= list_y && y <= list_y + list_height {
                        // Clicked inside list
                        let relative_y = y - list_y;
                        let index = (relative_y / item_h).floor() as usize;
                        if index < self.items.len() {
                            self.set_selected_index(index);
                            self.expanded = false;
                            return EventResult::ValueChanged(self.selected_index as f64);
                        }
                    } else if self.bounds.contains(x as f64, y as f64) {
                        // Clicked on header while expanded - close it
                        self.expanded = false;
                        return EventResult::Handled;
                    } else {
                        // Clicked outside? container handles this via hit_test failing usually
                        // But if we captured mouse, we might see it?
                        // If we are here, hit_test passed.
                        // If hit_test passed, we are either on header or overlay.
                    }
                } else {
                    // Toggle On
                    if self.bounds.contains(x as f64, y as f64) {
                        self.expanded = true;
                        self.hovered_item_index = Some(self.selected_index);
                        return EventResult::Handled;
                    }
                }
                EventResult::Handled
            }
            WidgetEvent::MouseMove { x, y } => {
                if self.expanded {
                    let item_h = self.item_height();
                    let list_height = self.items.len() as f32 * item_h;
                    let list_rect = Rect::new(
                        self.bounds.x,
                        self.bounds.y + self.bounds.height,
                        self.bounds.width,
                        list_height,
                    );

                    if list_rect.contains(*x, *y) {
                        let relative_y = *y as f32 - (self.bounds.y + self.bounds.height);
                        let index = (relative_y / item_h) as usize;
                        if index < self.items.len() {
                            self.hovered_item_index = Some(index);
                            return EventResult::Handled;
                        }
                    }
                    self.hovered_item_index = None;
                    return EventResult::Handled;
                }
                EventResult::NotHandled
            }
            WidgetEvent::MouseUp { .. } => {
                // Logic moved to MouseDown for immediate response, or kept here if drag-select desired.
                // For standard Dropdown, click-release on item selects it.
                // But MouseDown logic above handles it on down.
                // If we want MouseUp selection (drag to select), we'd need capture.
                // Assuming click-click behavior for now as per MouseDown implementation.
                EventResult::NotHandled
            }
            WidgetEvent::FocusLost => {
                self.expanded = false;
                self.focused = false;
                EventResult::Handled
            }
            WidgetEvent::FocusGained => {
                self.focused = true;
                EventResult::Handled
            }
            WidgetEvent::KeyDown { keycode, .. } => {
                if self.expanded {
                    match *keycode {
                        126 => {
                            // Up Arrow
                            if let Some(current) = self.hovered_item_index {
                                if current > 0 {
                                    self.hovered_item_index = Some(current - 1);
                                }
                            } else {
                                self.hovered_item_index = Some(self.items.len().saturating_sub(1));
                            }
                            EventResult::Handled
                        }
                        125 => {
                            // Down Arrow
                            if let Some(current) = self.hovered_item_index {
                                if current + 1 < self.items.len() {
                                    self.hovered_item_index = Some(current + 1);
                                }
                            } else if !self.items.is_empty() {
                                self.hovered_item_index = Some(0);
                            }
                            EventResult::Handled
                        }
                        36 => {
                            // Enter
                            if let Some(index) = self.hovered_item_index {
                                self.selected_index = index;
                                self.expanded = false;
                                EventResult::ValueChanged(self.selected_index as f64)
                            } else {
                                self.expanded = false;
                                EventResult::Handled
                            }
                        }
                        53 => {
                            // Escape
                            self.expanded = false;
                            EventResult::Handled
                        }
                        _ => EventResult::NotHandled,
                    }
                } else {
                    // Space (49) or Enter (36) or Down (125) to open?
                    match *keycode {
                        36 | 49 | 125 => {
                            self.expanded = true;
                            self.hovered_item_index = Some(self.selected_index);
                            EventResult::Handled
                        }
                        _ => EventResult::NotHandled,
                    }
                }
            }
            _ => EventResult::NotHandled,
        }
    }

    fn render(
        &self,
        shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        vulkan_context: &crate::VulkanContext,
        font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        // Render Header
        let bg_color = BACKGROUND_WIDGET;
        let border_color = if self.focused { BORDER_FOCUS } else { BORDER };

        shape_renderer.draw_rect(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            bg_color,
            4.0,
        );

        // Border
        shape_renderer.draw_rect(
            self.bounds.x - 1.0,
            self.bounds.y - 1.0,
            self.bounds.width + 2.0,
            self.bounds.height + 2.0,
            border_color,
            4.0,
        );

        // Text
        if let Some(text) = self.items.get(self.selected_index) {
            let _ = text_renderer.draw_text(
                vulkan_context,
                font_atlas,
                text,
                self.bounds.x + 10.0,
                self.bounds.y + self.bounds.height / 2.0 + 5.0, // Vert center
                16.0,
                TEXT_PRIMARY,
            );
        }

        // Arrow
        let arrow_x = self.bounds.x + self.bounds.width - 20.0;
        let arrow_y = self.bounds.y + self.bounds.height / 2.0;
        // Draw simple triangle or "v"
        // Since ShapeRenderer only does rects, we will draw a small rect for now or specialized logic?
        // Let's just draw a small rect as indicator
        shape_renderer.draw_rect(arrow_x, arrow_y - 2.0, 10.0, 4.0, TEXT_SECONDARY, 1.0);
    }

    fn render_overlay(
        &self,
        shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        vulkan_context: &crate::VulkanContext,
        font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        _screen_width: u32,
        _screen_height: u32,
    ) {
        if !self.expanded {
            return;
        }

        let item_h = self.item_height();
        let list_height = self.items.len() as f32 * item_h;
        let x = self.bounds.x;
        let y = self.bounds.y + self.bounds.height;
        let w = self.bounds.width;

        // Background
        shape_renderer.draw_rect(x, y, w, list_height, BACKGROUND_WIDGET, 4.0);

        // Border
        shape_renderer.draw_rect(x - 1.0, y - 1.0, w + 2.0, list_height + 2.0, BORDER, 4.0);

        // Items
        for (i, item) in self.items.iter().enumerate() {
            let item_y = y + i as f32 * item_h;

            // Highlight hover
            if Some(i) == self.hovered_item_index {
                shape_renderer.draw_rect(x, item_y, w, item_h, BACKGROUND_HOVER, 0.0);
            }

            // Highlight selected
            if i == self.selected_index {
                // Maybe a dot or diff text color?
                // For now, let's just use text color
            }

            let _ = text_renderer.draw_text(
                vulkan_context,
                font_atlas,
                item,
                x + 10.0,
                item_y + item_h / 2.0 + 5.0,
                16.0,
                if i == self.selected_index {
                    ACCENT
                } else {
                    TEXT_PRIMARY
                },
            );
        }
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
        self.focused
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn overlay_bounds(&self) -> Option<Rect> {
        if self.expanded {
            let item_h = self.item_height();
            let list_height = self.items.len() as f32 * item_h;
            Some(Rect::new(
                self.bounds.x,
                self.bounds.y + self.bounds.height,
                self.bounds.width,
                list_height,
            ))
        } else {
            None
        }
    }
}
