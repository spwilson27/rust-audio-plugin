use std::cell::RefCell;

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
    /// Cache of cumulative character widths (updated in render)
    layout_cache: RefCell<Vec<f32>>,
    is_dragging: bool,
    last_click_time: Option<std::time::Instant>,
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
            layout_cache: RefCell::new(Vec::new()),
            is_dragging: false,
            last_click_time: None,
        }
    }

    /// Set placeholder text
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Get placeholder text
    pub fn placeholder(&self) -> &str {
        &self.placeholder
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

    /// Delete word backward (Option+Backspace)
    fn delete_word_backward(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos == 0 {
            return;
        }

        let start = self.find_prev_boundary(self.cursor_pos);
        if start < self.cursor_pos {
            self.text.replace_range(start..self.cursor_pos, "");
            self.cursor_pos = start;
        }
    }

    /// Find previous word boundary (start of word)
    fn find_prev_boundary(&self, index: usize) -> usize {
        if index == 0 {
            return 0;
        }

        let chars: Vec<char> = self.text.chars().collect();
        let mut i = index;

        // Skip preceding whitespace
        while i > 0 && chars[i - 1].is_whitespace() {
            i -= 1;
        }

        // Skip preceding characters until whitespace
        while i > 0 && !chars[i - 1].is_whitespace() {
            i -= 1;
        }

        i
    }

    /// Find next word boundary (start of next word)
    fn find_next_boundary(&self, index: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let len = chars.len();
        if index >= len {
            return len;
        }

        let mut i = index;

        // Skip current word
        while i < len && !chars[i].is_whitespace() {
            i += 1;
        }

        // Skip whitespace
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }

        i
    }

    /// Move cursor word left
    fn move_cursor_word_left(&mut self, select: bool) {
        if select {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
        } else {
            self.selection_start = None;
        }

        self.cursor_pos = self.find_prev_boundary(self.cursor_pos);
    }

    /// Move cursor word right
    fn move_cursor_word_right(&mut self, select: bool) {
        if select {
            if self.selection_start.is_none() {
                self.selection_start = Some(self.cursor_pos);
            }
        } else {
            self.selection_start = None;
        }

        self.cursor_pos = self.find_next_boundary(self.cursor_pos);
    }

    /// Delete word forward (Option+Delete)
    fn delete_word_forward(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos >= self.text.len() {
            return;
        }

        let end = self.find_next_boundary(self.cursor_pos);
        if end > self.cursor_pos {
            self.text.replace_range(self.cursor_pos..end, "");
            // Cursor pos stays same
        }
    }

    /// Select all text
    fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.cursor_pos = self.text.len();
    }

    /// Find word boundaries around index
    fn find_word_boundaries(&self, index: usize) -> (usize, usize) {
        if self.text.is_empty() {
            return (0, 0);
        }

        let chars: Vec<char> = self.text.chars().collect();
        let len = chars.len();
        let index = index.min(len);

        // Find start
        let mut start = index;
        while start > 0 {
            let c = chars[start - 1];
            if c.is_whitespace() {
                break;
            }
            start -= 1;
        }

        // Find end
        let mut end = index;
        while end < len {
            let c = chars[end];
            if c.is_whitespace() {
                break;
            }
            end += 1;
        }

        (start, end)
    }

    /// Calculate cursor index from x coordinate
    fn get_cursor_index_from_x(&self, x: f32) -> usize {
        let text_x = self.bounds.x + 5.0;
        let relative_x = x - text_x;

        let cache = self.layout_cache.borrow();

        // If cache is valid
        if !cache.is_empty() {
            for i in 0..self.text.len() {
                let start = *cache.get(i).unwrap_or(&0.0);
                let end = *cache.get(i + 1).unwrap_or(&start);
                let center = (start + end) / 2.0;

                if relative_x < center {
                    return i;
                }
            }
        }
        self.text.len()
    }
}

impl Widget for Textbox {
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
            WidgetEvent::MouseDown { x, button, .. } => {
                if *button == 0 {
                    let now = std::time::Instant::now();
                    let is_double_click = if let Some(last) = self.last_click_time {
                        now.duration_since(last).as_millis() < 500
                    } else {
                        false
                    };
                    self.last_click_time = Some(now);

                    let x = *x as f32;
                    let new_pos = self.get_cursor_index_from_x(x);

                    if is_double_click {
                        let (start, end) = self.find_word_boundaries(new_pos);
                        self.selection_start = Some(start);
                        self.cursor_pos = end;
                        self.is_dragging = false; // Stop dragging on double click
                    } else {
                        self.cursor_pos = new_pos;
                        self.selection_start = None;
                        self.is_dragging = true;
                    }
                    EventResult::CaptureMouse
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::MouseMove { x, .. } => {
                if self.is_dragging {
                    let x = *x as f32;
                    let new_pos = self.get_cursor_index_from_x(x);

                    if self.selection_start.is_none() {
                        // Start selection from initial click position (roughly)
                        // Ideally we'd store the initial click index, but for now
                        // we can infer it or just start selection behavior
                        self.selection_start = Some(self.cursor_pos);
                    }
                    self.cursor_pos = new_pos;
                    EventResult::Handled
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::MouseUp { .. } => {
                if self.is_dragging {
                    self.is_dragging = false;
                    EventResult::ReleaseMouse
                } else {
                    EventResult::NotHandled
                }
            }
            WidgetEvent::KeyDown { keycode, modifiers } => {
                // macOS keycodes
                const LEFT_ARROW: u32 = 123;
                const RIGHT_ARROW: u32 = 124;
                const DELETE_KEY: u32 = 51; // Backspace
                const FWD_DELETE: u32 = 117; // Delete
                const HOME_KEY: u32 = 115;
                const END_KEY: u32 = 119;

                // Check for standard OS shortcuts
                if let Some(action) = crate::shortcuts::match_shortcut(*modifiers, *keycode) {
                    match action {
                        crate::shortcuts::StandardAction::Copy => {
                            self.copy();
                            return EventResult::Handled;
                        }
                        crate::shortcuts::StandardAction::Paste => {
                            self.paste();
                            return EventResult::ValueChanged(0.0);
                        }
                        crate::shortcuts::StandardAction::Cut => {
                            self.copy();
                            self.delete_selection();
                            return EventResult::ValueChanged(0.0);
                        }
                        crate::shortcuts::StandardAction::SelectAll => {
                            self.select_all();
                            return EventResult::Handled;
                        }
                        crate::shortcuts::StandardAction::DeleteWord => {
                            self.delete_word_backward();
                            return EventResult::ValueChanged(0.0);
                        }
                        crate::shortcuts::StandardAction::DeleteWordForward => {
                            self.delete_word_forward();
                            return EventResult::ValueChanged(0.0);
                        }
                    }
                }

                let shift = modifiers.contains(pal::Modifiers::SHIFT);
                let alt = modifiers.contains(pal::Modifiers::ALT);
                let meta = modifiers.contains(pal::Modifiers::META); // Cmd on Mac

                match *keycode {
                    LEFT_ARROW => {
                        if meta {
                            self.move_cursor_home(shift);
                        } else if alt {
                            self.move_cursor_word_left(shift);
                        } else {
                            self.move_cursor_left(shift);
                        }
                        EventResult::Handled
                    }
                    RIGHT_ARROW => {
                        if meta {
                            self.move_cursor_end(shift);
                        } else if alt {
                            self.move_cursor_word_right(shift);
                        } else {
                            self.move_cursor_right(shift);
                        }
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
                        // Option+Backspace handled by match_shortcut
                        self.handle_backspace();
                        EventResult::ValueChanged(0.0) // Text changed
                    }
                    FWD_DELETE => {
                        // Option+Delete handled by match_shortcut
                        self.handle_delete();
                        EventResult::ValueChanged(0.0)
                    }
                    _ => {
                        // Regular character input dealt with via TextInput event
                        EventResult::NotHandled
                    }
                }
            }
            WidgetEvent::TextInput(text) => {
                self.insert_text(text);
                EventResult::ValueChanged(0.0)
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
        use crate::theme::colors::*;

        // Colors
        let bg_color = BACKGROUND_WIDGET;

        let border_color = if self.focused { BORDER_FOCUS } else { BORDER };

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

        // Update layout cache
        {
            let mut cache = self.layout_cache.borrow_mut();
            cache.clear();
            cache.push(0.0);
            let mut x = 0.0;
            for c in self.text.chars() {
                let adv = _font_atlas.get_glyph_advance(c, text_size);
                x += adv;
                cache.push(x);
            }
        }

        if self.text.is_empty() && !self.placeholder.is_empty() && !self.focused {
            // Draw placeholder text
            let _ = _text_renderer.draw_text(
                _vulkan_context,
                _font_atlas,
                &self.placeholder,
                text_x,
                text_y,
                text_size,
                TEXT_DISABLED,
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
                TEXT_PRIMARY,
            );
        }

        // Draw selection
        if let Some(start) = self.selection_start {
            let cache = self.layout_cache.borrow();
            let (s, e) = if start < self.cursor_pos {
                (start, self.cursor_pos)
            } else {
                (self.cursor_pos, start)
            };

            // Safely get widths
            let total_width = *cache.last().unwrap_or(&0.0);
            let x1 = text_x + *cache.get(s).unwrap_or(&total_width);
            let x2 = text_x + *cache.get(e).unwrap_or(&total_width);

            if x2 > x1 {
                shape_renderer.draw_rect(
                    x1,
                    self.bounds.y + 4.0,
                    x2 - x1,
                    self.bounds.height - 8.0,
                    SELECTION,
                    1.0, // Slight rounding
                );
            }
        }

        // Draw cursor if focused
        if self.focused {
            let cache = self.layout_cache.borrow();
            let total_width = *cache.last().unwrap_or(&0.0);
            let cursor_offset = *cache.get(self.cursor_pos).unwrap_or(&total_width);

            let cursor_x = text_x + cursor_offset;
            let cursor_y = self.bounds.y + 5.0;
            let cursor_height = self.bounds.height - 10.0;

            shape_renderer.draw_rect(cursor_x, cursor_y, 2.0, cursor_height, CURSOR, 0.0);
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

    #[test]
    fn test_textbox_word_navigation() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello World Test");

        // Start at end (16)
        assert_eq!(textbox.cursor_position(), 16);

        // Move word left -> Start of "Test" (12)
        textbox.move_cursor_word_left(false);
        assert_eq!(textbox.cursor_position(), 12);

        // Move word left -> Start of "World" (6)
        textbox.move_cursor_word_left(false);
        assert_eq!(textbox.cursor_position(), 6);

        // Move word left -> Start of "Hello" (0)
        textbox.move_cursor_word_left(false);
        assert_eq!(textbox.cursor_position(), 0);

        // Move word right -> Start of "World" (6)
        textbox.move_cursor_word_right(false);
        assert_eq!(textbox.cursor_position(), 6);
    }

    #[test]
    fn test_textbox_delete_word_forward() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello World");
        textbox.cursor_pos = 0;

        // Delete "Hello " -> "World"
        textbox.delete_word_forward();
        assert_eq!(textbox.text(), "World");
        assert_eq!(textbox.cursor_position(), 0);
    }

    #[test]
    fn test_textbox_delete_word_backward_improved() {
        let mut textbox = Textbox::new(0.0, 0.0, 200.0, 30.0);
        textbox.set_text("Hello World"); // cursor at 11

        // Delete "World" -> "Hello "
        textbox.delete_word_backward();
        assert_eq!(textbox.text(), "Hello ");
        assert_eq!(textbox.cursor_position(), 6);

        // Delete "Hello " -> ""
        textbox.delete_word_backward();
        assert_eq!(textbox.text(), "");
    }
}
