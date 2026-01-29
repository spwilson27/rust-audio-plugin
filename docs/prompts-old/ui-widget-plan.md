# UI Widget Framework Implementation Plan

This plan outlines the implementation of a widget framework for the splug audio plugin, supporting textboxes, buttons, sliders, and knobs with full automated testing support.

## User Review Required

> [!IMPORTANT]
> This framework establishes core UI abstractions that will be used throughout the plugin. The design decisions below will affect future widget development and testing infrastructure.

**Key Design Decisions:**

1. **Event-Driven Architecture**: Widgets receive events from the [EventRouter](file:///Users/mrwilson/Software/rust-vst-2/pal/src/event_router.rs#14-18) (supports both hardware and RPC-injected events)
2. **Retained-Mode UI**: Widgets maintain their own state and handle their own rendering
3. **Coordinate System**: All widgets use screen-space pixel coordinates (top-left origin, matching Vulkan)
4. **State Management**: Widget state is local to the widget; parameter synchronization with audio thread happens at a higher level
5. **Testing Strategy**: All widgets support headless testing via RPC event injection

**Breaking Changes:**

- None (this is new functionality)

**Trade-offs:**

- **Retained vs Immediate Mode**: We chose retained mode for simplicity and compatibility with the existing event routing system. This means widgets maintain state between frames.
- **Focus Management**: Initial implementation will use simple tab-order focus management. More sophisticated focus systems (spatial navigation) can be added later.

## Proposed Changes

### Component: Core Widget Abstractions

#### [NEW] [gui/src/widgets/mod.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/mod.rs)

Main widget module that exports all widget types and core traits.

```rust
pub trait Widget {
    fn handle_event(&mut self, event: &WidgetEvent) -> EventResult;
    fn render(&self, renderer: &mut ShapeRenderer, screen_width: u32, screen_height: u32);
    fn bounds(&self) -> Rect;
    fn set_position(&mut self, x: f32, y: f32);
    fn is_focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
}

pub enum WidgetEvent {
    MouseDown { x: f64, y: f64, button: u32 },
    MouseUp { x: f64, y: f64, button: u32 },
    MouseMove { x: f64, y: f64 },
    MouseEnter,
    MouseExit,
    KeyDown { keycode: u32 },
    KeyUp { keycode: u32 },
    FocusGained,
    FocusLost,
}

pub enum EventResult {
    Handled,          // Event consumed, stop propagation
    NotHandled,       // Event not relevant, continue propagation
    ValueChanged(f64), // Widget value changed (for parameters)
}

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

**Rationale**: The `Widget` trait provides a uniform interface for all UI elements. `WidgetEvent` is a higher-level abstraction over raw `UIEvent` that includes widget-specific events like hover and focus changes. `EventResult` allows widgets to signal state changes to parent containers.

---

#### [NEW] [gui/src/widgets/button.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/button.rs)

A clickable button widget with hover and pressed states.

```rust
pub struct Button {
    bounds: Rect,
    label: String,
    state: ButtonState,
    on_click: Option<Box<dyn FnMut() + Send>>,
    focused: bool,
}

enum ButtonState {
    Normal,
    Hovered,
    Pressed,
}

impl Button {
    pub fn new(x: f32, y: f32, width: f32, height: f32, label: &str) -> Self;
    pub fn set_on_click<F>(&mut self, callback: F) where F: FnMut() + Send + 'static;
}

impl Widget for Button {
    // Implements hover detection, click handling, keyboard activation (Space/Enter)
}
```

**Features**:
- Visual feedback for hover/pressed states
- Keyboard activation when focused (Space or Enter keys)
- Callback-based click handling
- Accessible via RPC (click simulation)

---

#### [NEW] [gui/src/widgets/slider.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/slider.rs)

A horizontal or vertical slider widget for continuous value selection.

```rust
pub struct Slider {
    bounds: Rect,
    orientation: Orientation,
    value: f64,        // Normalized [0.0, 1.0]
    min_value: f64,
    max_value: f64,
    dragging: bool,
    focused: bool,
}

pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Slider {
    pub fn new(x: f32, y: f32, width: f32, height: f32, orientation: Orientation) -> Self;
    pub fn set_value(&mut self, value: f64);
    pub fn get_value(&self) -> f64;
    pub fn set_range(&mut self, min: f64, max: f64);
}

impl Widget for Slider {
    // Implements click-to-jump, drag-to-adjust, keyboard fine-tuning (arrow keys)
}
```

**Features**:
- Horizontal and vertical orientations
- Click-to-jump and drag-to-adjust
- Keyboard fine-tuning (arrow keys when focused)
- Normalized value [0.0, 1.0] with configurable display range
- Returns `EventResult::ValueChanged` when value updates

---

#### [NEW] [gui/src/widgets/knob.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/knob.rs)

A rotary knob widget (circular slider) commonly used in audio plugins.

```rust
pub struct Knob {
    center_x: f32,
    center_y: f32,
    radius: f32,
    value: f64,        // Normalized [0.0, 1.0]
    min_angle: f32,    // Radians (e.g., -2.35 for 270° range)
    max_angle: f32,    // Radians (e.g., 0.785)
    dragging: bool,
    drag_start_y: f64,
    focused: bool,
}

impl Knob {
    pub fn new(center_x: f32, center_y: f32, radius: f32) -> Self;
    pub fn set_value(&mut self, value: f64);
    pub fn get_value(&self) -> f64;
    pub fn set_angle_range(&mut self, min_radians: f32, max_radians: f32);
}

impl Widget for Knob {
    // Implements vertical drag-to-rotate, keyboard fine-tuning
}
```

**Features**:
- Vertical drag interaction (industry standard for rotary controls)
- Configurable angle range (e.g., 270° sweep)
- Visual arc indicator showing current value
- Keyboard fine-tuning (arrow keys)
- Returns `EventResult::ValueChanged` when value updates

---

#### [NEW] [gui/src/widgets/textbox.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/textbox.rs)

A single-line text input widget with selection and editing support.

```rust
pub struct Textbox {
    bounds: Rect,
    text: String,
    cursor_pos: usize,     // Byte position in string
    selection_start: Option<usize>,
    focused: bool,
    placeholder: String,
}

impl Textbox {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self;
    pub fn set_text(&mut self, text: &str);
    pub fn get_text(&self) -> &str;
    pub fn set_placeholder(&mut self, text: &str);
}

impl Widget for Textbox {
    // Implements text input, cursor movement, selection, copy/paste
}
```

**Features**:
- Text input via keyboard events
- Cursor movement (arrow keys, Home/End)
- Text selection (Shift+arrows)
- Copy/paste support (platform clipboard integration)
- Placeholder text when empty
- Auto-scroll for long text

**Testing Note**: For automated tests, we'll inject keyboard events via RPC to type text and verify the internal state.

---

### Component: Widget Container & Focus Management

#### [NEW] [gui/src/widgets/container.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs)

A container that manages multiple widgets, handles event routing, and manages focus.

```rust
pub struct WidgetContainer {
    widgets: Vec<Box<dyn Widget>>,
    focused_index: Option<usize>,
    hovered_index: Option<usize>,
}

impl WidgetContainer {
    pub fn new() -> Self;
    pub fn add_widget(&mut self, widget: Box<dyn Widget>);
    pub fn handle_ui_event(&mut self, event: UIEvent) -> Vec<EventResult>;
    pub fn render(&self, shape_renderer: &mut ShapeRenderer, screen_width: u32, screen_height: u32);
    pub fn focus_next(&mut self);
    pub fn focus_previous(&mut self);
    pub fn get_widget_mut(&mut self, index: usize) -> Option<&mut Box<dyn Widget>>;
}
```

**Responsibilities**:
- Event routing: Convert `UIEvent` to `WidgetEvent` and dispatch to appropriate widgets
- Hit testing: Determine which widget is under the mouse cursor
- Focus management: Handle Tab/Shift+Tab navigation between widgets
- Hover tracking: Send MouseEnter/MouseExit events when cursor moves between widgets
- Rendering coordination: Call [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) on all widgets in order

**Event Flow**:
1. `pal::MacOSWindow` receives OS event
2. [EventRouter](file:///Users/mrwilson/Software/rust-vst-2/pal/src/event_router.rs#14-18) routes `UIEvent` to container
3. `WidgetContainer::handle_ui_event()` converts to `WidgetEvent` and dispatches
4. Widget handles event and returns `EventResult`
5. Container propagates `ValueChanged` results to parent (for parameter updates)

---

### Component: Integration with Existing Systems

#### [MODIFY] [gui/src/lib.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs)

Export the new widget module and add widget rendering support to [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#76-84).

**Changes**:
- Add `pub mod widgets;` to expose the widget framework
- Modify [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#76-84) to own a `WidgetContainer`
- Update `GuiContext::handle_event()` to route events to the container
- Update `GuiContext::render()` to render widgets via the container

**Rationale**: This integrates the widget system with the existing GUI infrastructure without breaking existing code.

---

#### [MODIFY] [gui/src/vulkan/renderer.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/renderer.rs)

Add support for rendering widgets by exposing [ShapeRenderer](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/shape_renderer.rs#24-34) to external callers.

**Changes**:
- Add `pub fn get_shape_renderer(&mut self) -> &mut ShapeRenderer` method
- Modify [draw_frame()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/renderer.rs#236-406) to allow external code to record shape commands before presenting

**Rationale**: Widgets need access to [ShapeRenderer](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/shape_renderer.rs#24-34) to draw themselves. We expose it through the existing renderer to maintain encapsulation.

---

### Component: Testing Infrastructure

#### [NEW] [standalone/tests/widget_tests.rs](file:///Users/mrwilson/Software/rust-vst-2/standalone/tests/widget_tests.rs)

Integration tests for widget behavior using RPC event injection.

**Test Cases**:

1. **Button Click Test**
   - Inject MouseDown/MouseUp events via RPC
   - Verify callback is invoked
   - Verify visual state changes (captured via screenshot comparison)

2. **Slider Drag Test**
   - Inject MouseDown at slider track, MouseMove, MouseUp
   - Verify value updates correctly
   - Test keyboard arrow key adjustments

3. **Knob Rotation Test**
   - Inject vertical drag events (MouseDown, MouseMove, MouseUp)
   - Verify value changes proportionally to drag distance
   - Test keyboard fine-tuning

4. **Textbox Input Test**
   - Inject KeyDown events for typing "Hello"
   - Verify internal text state
   - Test cursor movement (arrow keys)
   - Test selection (Shift+arrow keys)
   - Verify text via screenshot or internal state query

5. **Focus Navigation Test**
   - Create multiple widgets
   - Inject Tab key events
   - Verify focus cycles correctly through widgets
   - Verify Shift+Tab reverses direction

**Testing Approach**:
```rust
#[test]
fn test_button_click() {
    // 1. Start standalone with --headless
    // 2. Connect RPC client
    // 3. Send MouseDown at button coordinates
    // 4. Send MouseUp
    // 5. Verify callback triggered (via internal state query or side effect)
    // 6. Capture screenshot and compare with golden image
}
```

---

#### [MODIFY] [definitions/debug_control.proto](file:///Users/mrwilson/Software/rust-vst-2/definitions/debug_control.proto)

Add RPC methods for querying widget state (for testing).

**New Methods**:
```protobuf
service DebugControl {
    // Existing methods...
    rpc SendInputEvent(InputEventMsg) returns (Ack);
    
    // New methods for widget testing
    rpc GetWidgetState(GetWidgetStateRequest) returns (GetWidgetStateResponse);
    rpc SetParameterValue(SetParameterRequest) returns (Ack);
}

message GetWidgetStateRequest {
    uint32 widget_id = 1;  // Index into widget container
}

message GetWidgetStateResponse {
    oneof state {
        ButtonState button = 1;
        SliderState slider = 2;
        KnobState knob = 3;
        TextboxState textbox = 4;
    }
}

message ButtonState {
    bool is_pressed = 1;
    bool is_hovered = 2;
}

message SliderState {
    double value = 1;
}

message KnobState {
    double value = 1;
}

message TextboxState {
    string text = 1;
    uint32 cursor_pos = 2;
}
```

**Rationale**: This allows tests to verify internal widget state without relying solely on visual output, making tests more robust and faster.

---

### Component: Documentation & Examples

#### [NEW] [gui/examples/widget_demo.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/examples/widget_demo.rs)

A runnable example demonstrating all widgets in a single window.

**Contents**:
- Create one instance of each widget type
- Layout in a grid
- Wire up callbacks to log interactions
- Demonstrate focus navigation

This serves as both documentation and a visual test harness for development.

---

## Verification Plan

### Automated Tests

1. **Unit Tests** (per widget):
   - Test state transitions (Normal → Hovered → Pressed for buttons)
   - Test value calculations (slider position to value mapping)
   - Test text editing operations (insert, delete, cursor movement)

2. **Integration Tests** (via RPC):
   - Run standalone in `--headless` mode
   - Connect RPC client
   - Inject events and verify state changes
   - Capture screenshots for visual regression testing
   - Execute all test cases listed in [Testing Infrastructure](#component-testing-infrastructure)

3. **Rendering Tests**:
   - Use golden image comparison (existing infrastructure from Phase 3.3)
   - Verify widgets render correctly at various states
   - Test at different DPI scales

### Manual Verification

1. **Standalone App Testing**:
   - Build and run [standalone](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#114-126) binary
   - Manually interact with each widget type
   - Verify visual feedback (hover, focus, pressed states)
   - Verify keyboard navigation works (Tab, Shift+Tab)

2. **Cross-Platform Testing**:
   - Verify widgets work on macOS (initial target)
   - Later: Verify on Windows (when Windows PAL is complete)

### Performance Verification

- Ensure widget rendering does not drop FPS below 60
- Profile event handling overhead (should be negligible)
- Verify widgets maintain 60 FPS during drag operations (slider/knob)

### Accessibility Verification

- Verify all widgets are keyboard-accessible
- Verify focus indicators are visible
- Verify color contrast meets minimum standards (can be tested programmatically)

---

## Implementation Sequence

### Phase 1: Core Abstractions (4-5 hours)
1. Create `widgets/mod.rs` with `Widget` trait and support types
2. Create `widgets/container.rs` with basic event routing
3. Integrate `WidgetContainer` into [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#76-84)
4. Write unit tests for event routing logic

### Phase 2: Simple Widgets (6-8 hours)
1. Implement `Button` widget with tests
2. Implement `Slider` widget with tests
3. Add RPC methods for state querying
4. Write integration tests for Button and Slider

### Phase 3: Complex Widgets (8-10 hours)
1. Implement `Knob` widget with rotary interaction
2. Implement `Textbox` widget with text editing
3. Add clipboard integration (platform-specific)
4. Write integration tests for Knob and Textbox

### Phase 4: Polish & Testing (4-6 hours)
1. Add focus indicators and visual polish
2. Create `widget_demo` example
3. Run full test suite and fix issues
4. Add golden image tests for all widgets
5. Document widget API and usage patterns

**Total Estimated Time**: 22-29 hours

---

## Open Questions

1. **Text Rendering**: Do we want to use the existing `text_renderer` module or does it need enhancements for cursor rendering and text selection highlighting?

2. **Clipboard Integration**: Should we use a cross-platform clipboard crate (like `arboard`) or implement platform-specific clipboard access directly?

3. **Widget IDs**: Do we need a formal widget ID system for RPC queries, or is indexing by position sufficient for testing?

4. **Layout System**: Should we implement a basic layout manager (e.g., flex/grid) or expect users to manually position widgets? (Can defer to future work)

5. **Color Scheme**: Should widgets have a hardcoded color scheme or should we define a theme system? (Recommend starting with hardcoded, add themes later)

---

## Dependencies

- Existing: [gui/src/vulkan/shape_renderer.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/shape_renderer.rs) (for drawing primitives)
- Existing: [gui/src/vulkan/text_renderer.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/text_renderer.rs) (for rendering text labels)
- Existing: [pal/src/event_router.rs](file:///Users/mrwilson/Software/rust-vst-2/pal/src/event_router.rs) (for event handling)
- Existing: `debug-server` (for RPC testing)
- New: Clipboard crate (TBD based on answer to question #2)

---

## Migration Path

This is new functionality, so there's no migration required. Existing code will continue to work. Widgets are opt-in.

Future work could migrate existing manual rendering code to use widgets for consistency.
