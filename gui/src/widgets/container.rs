//! Widget container for managing multiple widgets with event routing and layout

use std::collections::HashMap;

use super::{layout::Layout, EventResult, Widget, WidgetEvent, WidgetId};
use crate::vulkan::shape_renderer::ShapeRenderer;
use crate::vulkan::text_renderer::TextRenderer;
use pal::UIEvent;

/// Container for managing multiple widgets
///
/// Responsibilities:
/// - Event routing: Convert UIEvent to WidgetEvent and dispatch to widgets
/// - Hit testing: Determine which widget is under the mouse
/// - Focus management: Handle Tab/Shift+Tab navigation
/// - Hover tracking: Send MouseEnter/MouseExit events
/// - Layout: Position widgets using a layout manager
pub struct WidgetContainer {
    widgets: Vec<Box<dyn Widget>>,
    widget_map: HashMap<WidgetId, usize>, // ID -> index mapping
    focused_index: Option<usize>,
    hovered_index: Option<usize>,
    captured_index: Option<usize>, // Widget that has captured the mouse
    layout: Option<Box<dyn Layout>>,
    last_mouse_pos: (f64, f64),
}

impl WidgetContainer {
    /// Create a new empty widget container
    pub fn new() -> Self {
        Self {
            widgets: Vec::new(),
            widget_map: HashMap::new(),
            focused_index: None,
            hovered_index: None,
            captured_index: None,
            layout: None,
            last_mouse_pos: (0.0, 0.0),
        }
    }

    /// Set the layout manager for this container
    pub fn set_layout(&mut self, layout: Box<dyn Layout>) {
        self.layout = Some(layout);
    }

    /// Add a widget to the container
    ///
    /// The widget will be positioned according to the current layout manager.
    pub fn add_widget(&mut self, widget: Box<dyn Widget>) {
        let id = widget.id();
        let index = self.widgets.len();

        // Add to layout if present
        if let Some(layout) = &mut self.layout {
            let bounds = widget.bounds();
            layout.add_widget(bounds.width, bounds.height);
        }

        self.widgets.push(widget);
        self.widget_map.insert(id, index);
    }

    /// Get a widget by ID
    pub fn get_widget(&self, id: WidgetId) -> Option<&Box<dyn Widget>> {
        self.widget_map
            .get(&id)
            .and_then(|&index| self.widgets.get(index))
    }

    /// Get a mutable widget by ID
    pub fn get_widget_mut(&mut self, id: WidgetId) -> Option<&mut Box<dyn Widget>> {
        self.widget_map
            .get(&id)
            .and_then(|&index| self.widgets.get_mut(index))
    }

    /// Get a widget by index
    pub fn get_widget_at(&self, index: usize) -> Option<&Box<dyn Widget>> {
        self.widgets.get(index)
    }

    /// Get a mutable widget by index
    pub fn get_widget_at_mut(&mut self, index: usize) -> Option<&mut Box<dyn Widget>> {
        self.widgets.get_mut(index)
    }

    /// Apply the current layout to all widgets
    ///
    /// Should be called after adding widgets or when the container size changes.
    pub fn apply_layout(&mut self, width: f32, height: f32) {
        if let Some(layout) = &self.layout {
            let rects = layout.compute_bounds(width, height);

            for (widget, rect) in self.widgets.iter_mut().zip(rects.iter()) {
                widget.set_position(rect.x, rect.y);
            }
        }
    }

    /// Handle a UI event from the platform layer
    ///
    /// Returns a vector of EventResults from all widgets that handled the event.
    pub fn handle_ui_event(&mut self, event: UIEvent) -> Vec<EventResult> {
        match event {
            UIEvent::MouseDown { x, y, button } => {
                self.last_mouse_pos = (x, y);
                self.handle_mouse_down(x, y, button)
            }
            UIEvent::MouseUp { x, y, button } => {
                self.last_mouse_pos = (x, y);
                self.handle_mouse_up(x, y, button)
            }
            UIEvent::MouseMove { x, y } => {
                self.last_mouse_pos = (x, y);
                self.handle_mouse_move(x, y)
            }
            UIEvent::KeyDown { keycode } => self.handle_key_down(keycode),
            UIEvent::KeyUp { keycode } => self.handle_key_up(keycode),
            _ => Vec::new(), // Ignore other events
        }
    }

    fn handle_mouse_down(&mut self, x: f64, y: f64, button: u32) -> Vec<EventResult> {
        let mut results = Vec::new();

        // Hit test to find widget under cursor
        if let Some(widget_index) = self.hit_test(x, y) {
            // Update focus
            if self.focused_index != Some(widget_index) {
                self.clear_focus();
                self.set_focus(widget_index);
            }

            // Send event to widget
            let event = WidgetEvent::MouseDown { x, y, button };
            if let Some(widget) = self.widgets.get_mut(widget_index) {
                let result = widget.handle_event(&event);

                // Check if widget wants to capture mouse
                if matches!(result, EventResult::CaptureMouse) {
                    self.captured_index = Some(widget_index);
                }

                if !matches!(result, EventResult::NotHandled) {
                    results.push(result);
                }
            }
        } else {
            // Clicked outside all widgets - clear focus
            self.clear_focus();
        }

        results
    }

    fn handle_mouse_up(&mut self, x: f64, y: f64, button: u32) -> Vec<EventResult> {
        let mut results = Vec::new();

        // Send to captured widget first (if any), otherwise to hovered widget
        let target_index = self.captured_index.or(self.hovered_index);

        if let Some(widget_index) = target_index {
            let event = WidgetEvent::MouseUp { x, y, button };
            if let Some(widget) = self.widgets.get_mut(widget_index) {
                let result = widget.handle_event(&event);
                if !matches!(result, EventResult::NotHandled) {
                    results.push(result);
                }
            }
        }

        // Always release capture on mouse up
        self.captured_index = None;

        results
    }

    fn handle_mouse_move(&mut self, x: f64, y: f64) -> Vec<EventResult> {
        let mut results = Vec::new();

        // PRIORITY 1: If a widget has captured the mouse, send all events to it
        if let Some(captured_index) = self.captured_index {
            let event = WidgetEvent::MouseMove { x, y };
            if let Some(widget) = self.widgets.get_mut(captured_index) {
                let result = widget.handle_event(&event);
                if !matches!(result, EventResult::NotHandled) {
                    results.push(result);
                }
            }
            return results; // Don't update hover state while captured
        }

        // PRIORITY 2: Normal hover tracking
        let new_hovered = self.hit_test(x, y);

        // Handle hover state changes
        if new_hovered != self.hovered_index {
            // Send MouseExit to previously hovered widget
            if let Some(old_index) = self.hovered_index {
                if let Some(widget) = self.widgets.get_mut(old_index) {
                    widget.handle_event(&WidgetEvent::MouseExit);
                }
            }

            // Send MouseEnter to newly hovered widget
            if let Some(new_index) = new_hovered {
                if let Some(widget) = self.widgets.get_mut(new_index) {
                    widget.handle_event(&WidgetEvent::MouseEnter);
                }
            }

            self.hovered_index = new_hovered;
        }

        // Send MouseMove to hovered widget
        if let Some(widget_index) = self.hovered_index {
            let event = WidgetEvent::MouseMove { x, y };
            if let Some(widget) = self.widgets.get_mut(widget_index) {
                let result = widget.handle_event(&event);
                if !matches!(result, EventResult::NotHandled) {
                    results.push(result);
                }
            }
        }

        results
    }

    fn handle_key_down(&mut self, keycode: u32) -> Vec<EventResult> {
        let mut results = Vec::new();

        // Check for Tab (focus navigation)
        const TAB_KEY: u32 = 48; // macOS keycode for Tab
        if keycode == TAB_KEY {
            // TODO: Check for Shift modifier to go backwards
            self.focus_next();
            return results;
        }

        // Send to focused widget
        if let Some(widget_index) = self.focused_index {
            let event = WidgetEvent::KeyDown { keycode };
            if let Some(widget) = self.widgets.get_mut(widget_index) {
                let result = widget.handle_event(&event);
                if !matches!(result, EventResult::NotHandled) {
                    results.push(result);
                }
            }
        }

        results
    }

    fn handle_key_up(&mut self, keycode: u32) -> Vec<EventResult> {
        let mut results = Vec::new();

        // Send to focused widget
        if let Some(widget_index) = self.focused_index {
            let event = WidgetEvent::KeyUp { keycode };
            if let Some(widget) = self.widgets.get_mut(widget_index) {
                let result = widget.handle_event(&event);
                if !matches!(result, EventResult::NotHandled) {
                    results.push(result);
                }
            }
        }

        results
    }

    /// Hit test to find which widget contains the given point
    fn hit_test(&self, x: f64, y: f64) -> Option<usize> {
        // Test in reverse order (last added = topmost)
        for (index, widget) in self.widgets.iter().enumerate().rev() {
            if widget.bounds().contains(x, y) {
                return Some(index);
            }
        }
        None
    }

    /// Focus the next widget in tab order
    pub fn focus_next(&mut self) {
        if self.widgets.is_empty() {
            return;
        }

        let start_index = self
            .focused_index
            .map_or(0, |i| (i + 1) % self.widgets.len());
        let mut current_index = start_index;

        loop {
            if let Some(widget) = self.widgets.get(current_index) {
                if widget.can_focus() {
                    self.clear_focus();
                    self.set_focus(current_index);
                    return;
                }
            }

            current_index = (current_index + 1) % self.widgets.len();
            if current_index == start_index {
                break; // Went full circle, no focusable widgets
            }
        }
    }

    /// Focus the previous widget in tab order
    pub fn focus_previous(&mut self) {
        if self.widgets.is_empty() {
            return;
        }

        let start_index = self.focused_index.map_or(self.widgets.len() - 1, |i| {
            if i == 0 {
                self.widgets.len() - 1
            } else {
                i - 1
            }
        });
        let mut current_index = start_index;

        loop {
            if let Some(widget) = self.widgets.get(current_index) {
                if widget.can_focus() {
                    self.clear_focus();
                    self.set_focus(current_index);
                    return;
                }
            }

            current_index = if current_index == 0 {
                self.widgets.len() - 1
            } else {
                current_index - 1
            };

            if current_index == start_index {
                break; // Went full circle
            }
        }
    }

    fn set_focus(&mut self, index: usize) {
        if let Some(widget) = self.widgets.get_mut(index) {
            widget.set_focused(true);
            widget.handle_event(&WidgetEvent::FocusGained);
            self.focused_index = Some(index);
        }
    }

    fn clear_focus(&mut self) {
        if let Some(index) = self.focused_index {
            if let Some(widget) = self.widgets.get_mut(index) {
                widget.set_focused(false);
                widget.handle_event(&WidgetEvent::FocusLost);
            }
            self.focused_index = None;
        }
    }

    /// Render all widgets
    pub fn render(
        &self,
        shape_renderer: &mut ShapeRenderer,
        text_renderer: &mut TextRenderer,
        vulkan_context: &crate::VulkanContext,
        font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
        screen_width: u32,
        screen_height: u32,
    ) {
        for widget in &self.widgets {
            widget.render(
                shape_renderer,
                text_renderer,
                vulkan_context,
                font_atlas,
                screen_width,
                screen_height,
            );
        }
    }

    /// Get the number of widgets in this container
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Check if the container is empty
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }
}

impl Default for WidgetContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::Rect;

    // Mock widget for testing
    struct MockWidget {
        id: WidgetId,
        bounds: Rect,
        focused: bool,
        can_focus: bool,
        requests_capture: bool,
        events_received: Vec<String>,
    }

    impl MockWidget {
        fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
            Self {
                id: WidgetId::new(),
                bounds: Rect::new(x, y, width, height),
                focused: false,
                can_focus: true,
                requests_capture: false,
                events_received: Vec::new(),
            }
        }

        fn non_focusable(mut self) -> Self {
            self.can_focus = false;
            self
        }

        fn with_capture(mut self) -> Self {
            self.requests_capture = true;
            self
        }
    }

    impl Widget for MockWidget {
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
            self.events_received.push(format!("{:?}", event));
            if self.requests_capture && matches!(event, WidgetEvent::MouseDown { .. }) {
                EventResult::CaptureMouse
            } else {
                EventResult::Handled
            }
        }

        fn render(
            &self,
            _shape_renderer: &mut ShapeRenderer,
            _text_renderer: &mut TextRenderer,
            _vulkan_context: &crate::VulkanContext,
            _font_atlas: &mut crate::vulkan::text_renderer::FontAtlas,
            _screen_width: u32,
            _screen_height: u32,
        ) {
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
            self.can_focus
        }
    }

    #[test]
    fn test_container_add_widget() {
        let mut container = WidgetContainer::new();
        let widget = Box::new(MockWidget::new(10.0, 20.0, 50.0, 30.0));
        let id = widget.id();

        container.add_widget(widget);

        assert_eq!(container.len(), 1);
        assert!(container.get_widget(id).is_some());
    }

    #[test]
    fn test_hit_testing() {
        let mut container = WidgetContainer::new();
        container.add_widget(Box::new(MockWidget::new(10.0, 10.0, 50.0, 50.0)));
        container.add_widget(Box::new(MockWidget::new(100.0, 100.0, 50.0, 50.0)));

        // Hit first widget
        assert_eq!(container.hit_test(30.0, 30.0), Some(0));

        // Hit second widget
        assert_eq!(container.hit_test(120.0, 120.0), Some(1));

        // Hit neither
        assert_eq!(container.hit_test(200.0, 200.0), None);
    }

    #[test]
    fn test_focus_navigation() {
        let mut container = WidgetContainer::new();
        container.add_widget(Box::new(MockWidget::new(0.0, 0.0, 50.0, 50.0)));
        container.add_widget(Box::new(MockWidget::new(0.0, 60.0, 50.0, 50.0)));
        container.add_widget(Box::new(MockWidget::new(0.0, 120.0, 50.0, 50.0)));

        // Initially no focus
        assert!(container.focused_index.is_none());

        // Focus next (should go to widget 0)
        container.focus_next();
        assert_eq!(container.focused_index, Some(0));
        assert!(container.get_widget_at(0).unwrap().is_focused());

        // Focus next again (should go to widget 1)
        container.focus_next();
        assert_eq!(container.focused_index, Some(1));
        assert!(!container.get_widget_at(0).unwrap().is_focused());
        assert!(container.get_widget_at(1).unwrap().is_focused());

        // Focus previous (back to widget 0)
        container.focus_previous();
        assert_eq!(container.focused_index, Some(0));
    }

    #[test]
    fn test_focus_skips_non_focusable() {
        let mut container = WidgetContainer::new();
        container.add_widget(Box::new(MockWidget::new(0.0, 0.0, 50.0, 50.0)));
        container.add_widget(Box::new(
            MockWidget::new(0.0, 60.0, 50.0, 50.0).non_focusable(),
        ));
        container.add_widget(Box::new(MockWidget::new(0.0, 120.0, 50.0, 50.0)));

        // Focus next - should skip widget 1
        container.focus_next();
        assert_eq!(container.focused_index, Some(0));

        container.focus_next();
        assert_eq!(container.focused_index, Some(2)); // Skipped 1
    }

    #[test]
    fn test_mouse_down_sets_focus() {
        let mut container = WidgetContainer::new();
        container.add_widget(Box::new(MockWidget::new(10.0, 10.0, 50.0, 50.0)));
        container.add_widget(Box::new(MockWidget::new(100.0, 100.0, 50.0, 50.0)));

        // Click second widget
        container.handle_ui_event(UIEvent::MouseDown {
            x: 120.0,
            y: 120.0,
            button: 0,
        });

        assert_eq!(container.focused_index, Some(1));
    }

    #[test]
    fn test_hover_tracking() {
        let mut container = WidgetContainer::new();
        container.add_widget(Box::new(MockWidget::new(10.0, 10.0, 50.0, 50.0)));

        // Move mouse into widget
        container.handle_ui_event(UIEvent::MouseMove { x: 30.0, y: 30.0 });
        assert_eq!(container.hovered_index, Some(0));

        // Move mouse out
        container.handle_ui_event(UIEvent::MouseMove { x: 200.0, y: 200.0 });
        assert_eq!(container.hovered_index, None);
    }

    #[test]
    fn test_mouse_capture() {
        let mut container = WidgetContainer::new();
        // Widget 0: Standard, no capture
        container.add_widget(Box::new(MockWidget::new(10.0, 10.0, 50.0, 50.0)));
        // Widget 1: Requests capture on MouseDown
        container.add_widget(Box::new(
            MockWidget::new(100.0, 100.0, 50.0, 50.0).with_capture(),
        ));

        // 1. Hover over capture widget
        container.handle_ui_event(UIEvent::MouseMove { x: 120.0, y: 120.0 });
        assert_eq!(container.hovered_index, Some(1));

        // 2. Click capture widget (MouseDown)
        container.handle_ui_event(UIEvent::MouseDown {
            x: 120.0,
            y: 120.0,
            button: 0,
        });
        // Should capture
        assert_eq!(container.captured_index, Some(1));

        // 3. Move mouse outside widget
        // Even though it's over widget 0 (30,30), widget 1 should still receive the event
        // and hover state should NOT update to widget 0 because of capture
        container.handle_ui_event(UIEvent::MouseMove { x: 30.0, y: 30.0 });

        // Assert capture is still held
        assert_eq!(container.captured_index, Some(1));

        // Verify widget 1 received the Move event
        let widget = container.widgets[1]
            .as_any()
            .downcast_ref::<MockWidget>()
            .unwrap();
        let last_event = widget.events_received.last().unwrap();
        assert!(
            last_event.contains("MouseMove"),
            "Captured widget should receive MouseMove"
        );
        assert!(
            last_event.contains("x: 30.0"),
            "Coordinate should match event"
        );

        assert_eq!(
            container.hovered_index,
            Some(1),
            "Hover index should not change during capture"
        );

        // 4. Release mouse
        container.handle_ui_event(UIEvent::MouseUp {
            x: 30.0,
            y: 30.0,
            button: 0,
        });
        assert_eq!(
            container.captured_index, None,
            "Capture should be released on MouseUp"
        );
    }

    #[test]
    fn test_knob_single_click_drag() {
        use crate::widgets::Knob;
        let mut container = WidgetContainer::new();
        // Knob at 10,10 size 30x30. Center approx 25,25.
        // Sensitivity 1.0 (default?)
        let knob = Knob::new(10.0, 10.0, 30.0);
        container.add_widget(Box::new(knob));

        // 1. Mouse Down on Knob (25, 25). Unfocused.
        // This should set focus AND capture mouse.
        let _ = container.handle_ui_event(UIEvent::MouseDown {
            x: 25.0,
            y: 25.0,
            button: 0,
        });

        // Assert capture
        assert_eq!(container.captured_index, Some(0));

        // 2. Drag Up (Decrease y) by 10 pixels
        // Knob sensitivity is usually pixels -> value.
        // Let's drag a significant amount.
        let results = container.handle_ui_event(UIEvent::MouseMove { x: 25.0, y: 15.0 });
        let value_changed = results
            .iter()
            .any(|r| matches!(r, EventResult::ValueChanged(_)));
        assert!(
            value_changed,
            "Should produce ValueChanged event on drag step 1"
        );

        // 3. More Drag
        let results = container.handle_ui_event(UIEvent::MouseMove { x: 25.0, y: 5.0 });
        let value_changed = results
            .iter()
            .any(|r| matches!(r, EventResult::ValueChanged(_)));
        assert!(
            value_changed,
            "Should produce ValueChanged event on drag step 2"
        );

        // 4. Mouse Up
        container.handle_ui_event(UIEvent::MouseUp {
            x: 25.0,
            y: 5.0,
            button: 0,
        });
        assert_eq!(container.captured_index, None);
    }
}
