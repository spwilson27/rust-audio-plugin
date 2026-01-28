# TODOs


# UI Widget Framework - Task Breakdown

## Phase 1: Core Abstractions
- [x] Create [gui/src/widgets/mod.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/mod.rs)
  - [x] Define [Widget](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/mod.rs#20-54) trait with core methods (including [id()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs#399-402) method)
  - [x] Define `WidgetEvent` enum
  - [x] Define `EventResult` enum
  - [x] Define [Rect](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/mod.rs#102-108) struct for bounds
  - [x] Add module exports
- [x] Create [gui/src/widgets/widget_id.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/widget_id.rs)
  - [x] Implement [WidgetId](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/widget_id.rs#13-14) with atomic counter
  - [x] Add [new()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/layout.rs#31-36), [from_raw()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/widget_id.rs#25-31), and [as_u64()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/widget_id.rs#32-36) methods
  - [x] Add tests for ID uniqueness
- [x] Create [gui/src/widgets/layout.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/layout.rs)
  - [x] Define [Layout](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/layout.rs#9-21) trait
  - [x] Implement [ManualLayout](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/layout.rs#26-29)
  - [x] Implement [GridLayout](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/layout.rs#71-78)
  - [x] Implement [FlexLayout](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/layout.rs#144-152) with flexbox-like behavior
  - [x] Add unit tests for each layout type
- [x] Create [gui/src/widgets/container.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs)
  - [x] Implement [WidgetContainer](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs#18-26) struct with ID-based lookup
  - [x] Implement event routing (UIEvent → WidgetEvent)
  - [x] Implement hit testing for mouse events
  - [x] Implement focus management (Tab/Shift+Tab)
  - [x] Implement hover tracking (MouseEnter/MouseExit)
  - [x] Integrate layout system for widget positioning
- [x] Integrate with [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#79-87)
  - [x] Add [WidgetContainer](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs#18-26) field to [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#79-87)
  - [ ] Update [handle_event()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs#403-407) to route to container
  - [ ] Update [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) to render widgets with layout
- [x] Write unit tests
  - [x] Test hit testing logic
  - [x] Test focus navigation
  - [x] Test event routing
  - [x] Test widget ID lookup
  - [x] Test layout calculations

## Phase 2: Simple Widgets
- [ ] Implement `Button` widget
  - [ ] Create `gui/src/widgets/button.rs`
  - [ ] Implement state machine (Normal/Hovered/Pressed)
  - [ ] Implement mouse event handling
  - [ ] Implement keyboard activation (Space/Enter)
  - [ ] Implement rendering (using ShapeRenderer)
  - [ ] Add callback support
  - [ ] Write unit tests
- [ ] Implement `Slider` widget
  - [ ] Create `gui/src/widgets/slider.rs`
  - [ ] Support horizontal and vertical orientations
  - [ ] Implement click-to-jump behavior
  - [ ] Implement drag-to-adjust behavior
  - [ ] Implement keyboard fine-tuning (arrow keys)
  - [ ] Implement value normalization [0.0, 1.0]
  - [ ] Implement rendering (track + thumb)
  - [ ] Write unit tests
- [ ] Extend RPC for testing
  - [ ] Modify `debug_control.proto` to add GetWidgetState (uses WidgetId)
  - [ ] Add SetWidgetValue RPC method
  - [ ] Implement GetWidgetState in `debug-server`
  - [ ] Add widget state message types (ButtonState, SliderState, etc.)
- [ ] Write integration tests
  - [ ] Test button click via RPC (lookup by ID)
  - [ ] Test slider drag via RPC
  - [ ] Test keyboard navigation
  - [ ] Test widgets in different layouts (Grid, Flex)
  - [ ] Add golden image tests

## Phase 3: Complex Widgets
- [ ] Enhance text rendering
  - [ ] Modify [gui/src/vulkan/text_renderer.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/vulkan/text_renderer.rs)
  - [ ] Add `draw_cursor()` method
  - [ ] Add `draw_selection()` method
  - [ ] Add `get_cursor_position()` helper
  - [ ] Add `get_char_index_at_position()` helper
  - [ ] Write tests for new text rendering features
- [ ] Implement `Knob` widget
  - [ ] Create `gui/src/widgets/knob.rs`
  - [ ] Implement vertical drag interaction
  - [ ] Implement value-to-angle mapping
  - [ ] Implement rendering (arc + indicator)
  - [ ] Implement keyboard fine-tuning
  - [ ] Write unit tests
- [ ] Add clipboard support
  - [ ] Add `arboard` crate to [gui/Cargo.toml](file:///Users/mrwilson/Software/rust-vst-2/gui/Cargo.toml)
  - [ ] Create clipboard wrapper module
  - [ ] Test clipboard on macOS
- [ ] Implement `Textbox` widget
  - [ ] Create `gui/src/widgets/textbox.rs`
  - [ ] Implement text storage and cursor position
  - [ ] Implement text input handling
  - [ ] Implement cursor movement (arrow keys, Home/End)
  - [ ] Implement text selection (Shift+arrows)
  - [ ] Implement copy/paste using arboard
  - [ ] Implement rendering using enhanced text_renderer
  - [ ] Add placeholder text support
  - [ ] Write unit tests

  - [ ] Implement platform-specific clipboard access
  - [ ] Add tests for copy/paste
- [ ] Write integration tests
  - [ ] Test knob rotation via RPC
  - [ ] Test textbox typing via RPC
  - [ ] Test text selection and editing
  - [ ] Add golden image tests

## Phase 4: Polish & Testing
- [ ] Visual polish
  - [ ] Define hardcoded color palette (background, text, accent, etc.)
  - [ ] Add focus indicators (outline/glow)
  - [ ] Apply consistent color scheme to all widgets
  - [ ] Add hover feedback for all interactive elements
  - [ ] Test at different DPI scales
- [ ] Create demo example
  - [ ] Create `gui/examples/widget_demo.rs`
  - [ ] Demonstrate ManualLayout with custom positioning
  - [ ] Demonstrate GridLayout with all widgets
  - [ ] Demonstrate FlexLayout for responsive design
  - [ ] Wire up callbacks to log interactions
  - [ ] Add instructions for keyboard navigation
- [ ] Complete test suite
  - [ ] Run all unit tests and fix failures
  - [ ] Run all integration tests and fix failures
  - [ ] Run golden image tests and update images
  - [ ] Test focus navigation edge cases
  - [ ] Performance profiling (ensure 60 FPS)
- [ ] Documentation
  - [ ] Document Widget trait API
  - [ ] Document each widget's public interface
  - [ ] Add usage examples to module docs
  - [ ] Update main README with widget framework info

## Approved Decisions (User Confirmed)
- [x] Text rendering: Enhance existing text_renderer for cursor/selection
- [x] Clipboard: Use `arboard` crate
- [x] Widget IDs: Implement formal widget ID system
- [x] Layout system: Implement flexible layout manager (Manual, Grid, Flex)
- [x] Color scheme: Use hardcoded colors (theme system deferred)
