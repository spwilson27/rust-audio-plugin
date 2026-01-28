//! Textbox widget - single-line text input

use super::{EventResult, Rect, Widget, WidgetEvent, WidgetId};
use crate::clipboard::ClipboardManager;
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;

/// Textbox widget for text input
///
/// Features:
/// - Single-line text editing
/// - Cursor navigation (arrow keys, Home/End)
/// - Text selection (Shift+arrows)
/// - Copy/paste support
/// - Placeholder text
pub struct Textbox {
    id: WidgetId,
    bounds: Rect,
    text: String,
    cursor_pos: usize,              // Cursor position in characters
    selection_start: Option<usize>, // Start of selection (if any)
    focused: bool,
    placeholder: String,
    clipboard: Option<ClipboardManager>,
}

impl Textbox {
    /// Create a new textbox
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::new(),
            bounds: Rect::new(x, y, width, height),
            text: String::new(),
            cursor_pos: 0,
            selection_start: None,
            focused: false,
            placeholder: String::new(),
            clipboard: ClipboardManager::new().ok(),
        }
    }

    /// Set placeholder text
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Get the current text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor_pos = self.text.len(); // Move cursor to end
        self.selection_start = None;
    }

    /// Get cursor position
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Check if there's a selection
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
    }

    /// Get the selected text (if any)
    pub fn selected_text(&self) -> Option<&str> {
        if let Some(start) = self.selection_start {
            let (sel_start, sel_end) = if start < self.cursor_pos {
                (start, self.cursor_pos)
            } else {
                (self.cursor_pos, start)
            };
            Some(&self.text[sel_start..sel_end])
        } else {
            None
        }
    }

    /// Insert text at cursor
    fn insert_text(&mut self, s: &str) {
        // Delete selection if any
        if self.has_selection() {
            self.delete_selection();
        }

        self.text.insert_str(self.cursor_pos, s);
        self.cursor_pos += s.len();
        self.selection_start = None;
    }

    /// Delete selected text
    fn delete_selection(&mut self) {
        if let Some(start) = self.selection_start {
            let (sel_start, sel_end) = if start < self.cursor_pos {
                (start, self.cursor_pos)
            } else {
                (self.cursor_pos, start)
            };
            self.text.replace_range(sel_start..sel_end, "");
            self.cursor_pos = sel_start;
            self.selection_start = None;
        }
    }

    /// Move cursor left
    fn move_cursor_left(&mut self, shift: bool) {
        if shift {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
        } else {
            self.selection_start = None;
        }

        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        }
    }

    /// Move cursor right
    fn move_cursor_right(&mut self, shift: bool) {
        if shift {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
        } else {
            self.selection_start = None;
        }

        if self.cursor_pos < self.text.len() {
            self.cursor_pos += 1;
        }
    }

    /// Move cursor to start
    fn move_cursor_home(&mut self, shift: bool) {
        if shift {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
        } else {
            self.selection_start = None;
        }
        self.cursor_pos = 0;
    }

    /// Move cursor to end
    fn move_cursor_end(&mut self, shift: bool) {
        if shift {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
        } else {
            self.selection_start = None;
        }
        self.cursor_pos = self.text.len();
    }

    /// Handle backspace
    fn handle_backspace(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else if self.cursor_pos > 0 {
            self.text.remove(self.cursor_pos - 1);
            self.cursor_pos -= 1;
        }
    }

    /// Handle delete
    fn handle_delete(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else if self.cursor_pos < self.text.len() {
            self.text.remove(self.cursor_pos);
        }
    }

    /// Copy selected text to clipboard
    fn copy(&mut self) {
        // Get selected text first to avoid borrow conflicts
        let text_to_copy = self.selected_text().map(|s| s.to_string());

        if let Some(text) = text_to_copy {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
    }

    /// Paste from clipboard
    fn paste(&mut self) {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                self.insert_text(&text);
            }
        }
    }

    /// Select all text
    fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor_pos = self.text.len();
    }
}

impl Widget for Textbox {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn handle_event(&mut self, event: &WidgetEvent) -> EventResult {
        match event {
            WidgetEvent::MouseDown { x: _, .. } => {
                // TODO: Calculate cursor position from click position
                // For now, just move to end
                self.cursor_pos = self.text.len();
                self.selection_start = None;
                EventResult::Handled
            }
            WidgetEvent::KeyDown { keycode } => {
                // macOS keycodes
                const LEFT_ARROW: u32 = 123;
                const RIGHT_ARROW: u32 = 124;
                const DELETE_KEY: u32 = 51; // Backspace
                const FWD_DELETE: u32 = 117; // Delete
                const HOME_KEY: u32 = 115;
                const END_KEY: u32 = 119;
                const A_KEY: u32 = 0; // Cmd+A (select all)
                const C_KEY: u32 = 8; // Cmd+C (copy)
                const V_KEY: u32 = 9; // Cmd+V (paste)
                const X_KEY: u32 = 7; // Cmd+X (cut)

                // TODO: Detect modifier keys (Shift, Cmd, Ctrl)
                // For now, assuming no modifiers
                let shift = false;
                let cmd = false;

                match *keycode {
                    LEFT_ARROW => {
                        self.move_cursor_left(shift);
                        EventResult::Handled
                    }
                    RIGHT_ARROW => {
                        self.move_cursor_right(shift);
                        EventResult::Handled
                    }
                    HOME_KEY => {
                        self.move_cursor_home(shift);
                        EventResult::Handled
                    }
                    END_KEY => {
                        self.move_cursor_end(shift);
                        EventResult::Handled
                    }
                    DELETE_KEY => {
                        self.handle_backspace();
                        EventResult::ValueChanged(0.0) // Text changed
                    }
                    FWD_DELETE => {
                        self.handle_delete();
                        EventResult::ValueChanged(0.0)
                    }
                    A_KEY if cmd => {
                        self.select_all();
                        EventResult::Handled
                    }
                    C_KEY if cmd => {
                        self.copy();
                        EventResult::Handled
                    }
                    V_KEY if cmd => {
                        self.paste();
                        EventResult::ValueChanged(0.0)
                    }
                    X_KEY if cmd => {
                        self.copy();
                        self.delete_selection();
                        EventResult::ValueChanged(0.0)
                    }
                    _ => {
                        // Regular character input
                        // TODO: Convert keycode to character
                        // For now, just return NotHandled
                        EventResult::NotHandled
                    }
                }
            }
            WidgetEvent::FocusGained => EventResult::Handled,
            WidgetEvent::FocusLost => {
                self.selection_start = None;
                EventResult::Handled
            }
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
        // Colors
        let bg_color = if self.focused {
            [0.2, 0.2, 0.25, 1.0] // Slightly lighter when focused
        } else {
            [0.15, 0.15, 0.15, 1.0]
        };
        let border_color = if self.focused {
            [0.4, 0.6, 0.8, 1.0] // Blue border when focused
        } else {
            [0.3, 0.3, 0.3, 1.0]
        };
        let selection_color = [0.2, 0.4, 0.7, 0.5]; // Semi-transparent blue

        // Draw background
        shape_renderer.draw_rect(
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            bg_color,
            2.0,
        );

        // Draw border
        shape_renderer.draw_rect(
            self.bounds.x - 1.0,
            self.bounds.y - 1.0,
            self.bounds.width + 2.0,
            self.bounds.height + 2.0,
            border_color,
            2.0,
        );

        // Text rendering
        let text_x = self.bounds.x + 5.0; // Left padding
        let text_y = self.bounds.y + self.bounds.height / 2.0 + 6.0; // Vertical center
        let text_size = 16.0;

        if self.text.is_empty() && !self.placeholder.is_empty() && !self.focused {
            // Draw placeholder text
            let _ = _text_renderer.draw_text(
                _vulkan_context,
                _font_atlas,
                &self.placeholder,
                text_x,
                text_y,
                text_size,
                [0.5, 0.5, 0.5, 1.0], // Gray placeholder
            );
        } else if !self.text.is_empty() {
            // Draw actual text
            let _ = _text_renderer.draw_text(
                _vulkan_context,
                _font_atlas,
                &self.text,
                text_x,
                text_y,
                text_size,
                [0.9, 0.9, 0.9, 1.0], // White text
            );
        }

        // TODO: Draw selection highlight if has_selection()
        // This would require calculating text width up to selection points

        // Draw cursor if focused
        if self.focused {
            // TODO: Calculate cursor X position based on text width up to cursor_pos
            // For now, use a simple approximation
            let cursor_x = text_x + (self.cursor_pos as f32 * 8.0); // Approximate character width
            let cursor_y = self.bounds.y + 5.0;
            let cursor_height = self.bounds.height - 10.0;

            shape_renderer.draw_rect(
                cursor_x,
                cursor_y,
                2.0,
                cursor_height,
                [1.0, 1.0, 1.0, 0.8], // White cursor
                0.0,
            );
        }

        let _ = selection_color; // Suppress unused warning
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
    fn test_textbox_creation() {
        let textbox = Textbox::new(10.0, 20.0, 200.0, 30.0);
        assert_eq!(textbox.text(), "");
        assert_eq!(textbox.cursor_position(), 0);
        assert!(!textbox.has_selection());
    }

    #[test]
    fn test_textbox_set_text() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");
        assert_eq!(textbox.text(), "Hello");
        assert_eq!(textbox.cursor_position(), 5);
    }

    #[test]
    fn test_textbox_insert() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.insert_text("Hello");
        assert_eq!(textbox.text(), "Hello");
        assert_eq!(textbox.cursor_position(), 5);

        textbox.cursor_pos = 0;
        textbox.insert_text("X");
        assert_eq!(textbox.text(), "XHello");
    }

    #[test]
    fn test_textbox_cursor_movement() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");

        // Move left
        textbox.move_cursor_left(false);
        assert_eq!(textbox.cursor_position(), 4);

        // Move right
        textbox.move_cursor_right(false);
        assert_eq!(textbox.cursor_position(), 5);

        // Home
        textbox.move_cursor_home(false);
        assert_eq!(textbox.cursor_position(), 0);

        // End
        textbox.move_cursor_end(false);
        assert_eq!(textbox.cursor_position(), 5);
    }

    #[test]
    fn test_textbox_selection() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");
        textbox.cursor_pos = 0;

        // Select with shift
        textbox.move_cursor_right(true);
        textbox.move_cursor_right(true);
        assert!(textbox.has_selection());
        assert_eq!(textbox.selected_text(), Some("He"));
    }

    #[test]
    fn test_textbox_backspace() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");

        textbox.handle_backspace();
        assert_eq!(textbox.text(), "Hell");
        assert_eq!(textbox.cursor_position(), 4);
    }

    #[test]
    fn test_textbox_delete() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");
        textbox.cursor_pos = 0;

        textbox.handle_delete();
        assert_eq!(textbox.text(), "ello");
        assert_eq!(textbox.cursor_position(), 0);
    }

    #[test]
    fn test_textbox_delete_selection() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");
        textbox.cursor_pos = 0;
        textbox.move_cursor_right(true);
        textbox.move_cursor_right(true);

        textbox.delete_selection();
        assert_eq!(textbox.text(), "llo");
        assert_eq!(textbox.cursor_position(), 0);
        assert!(!textbox.has_selection());
    }

    #[test]
    fn test_textbox_select_all() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello");

        textbox.select_all();
        assert!(textbox.has_selection());
        assert_eq!(textbox.selected_text(), Some("Hello"));
    }

    #[test]
    fn test_textbox_placeholder() {
        let textbox = Textbox::new(0.0, 0.0, 200.0, 30.0).with_placeholder("Enter text...");
        assert_eq!(textbox.placeholder, "Enter text...");
    }
}
