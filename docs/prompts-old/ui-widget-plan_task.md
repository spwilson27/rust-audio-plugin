# UI Widget Framework - Task Breakdown

## Phase 1: Core Abstractions
- [ ] Create `gui/src/widgets/mod.rs`
  - [ ] Define `Widget` trait with core methods
  - [ ] Define `WidgetEvent` enum
  - [ ] Define `EventResult` enum
  - [ ] Define `Rect` struct for bounds
  - [ ] Add module exports
- [ ] Create `gui/src/widgets/container.rs`
  - [ ] Implement `WidgetContainer` struct
  - [ ] Implement event routing (UIEvent → WidgetEvent)
  - [ ] Implement hit testing for mouse events
  - [ ] Implement focus management (Tab/Shift+Tab)
  - [ ] Implement hover tracking (MouseEnter/MouseExit)
- [ ] Integrate with [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#76-84)
  - [ ] Add `WidgetContainer` field to [GuiContext](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#76-84)
  - [ ] Update [handle_event()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#127-132) to route to container
  - [ ] Update [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) to render widgets
- [ ] Write unit tests
  - [ ] Test hit testing logic
  - [ ] Test focus navigation
  - [ ] Test event routing

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
  - [ ] Modify `debug_control.proto` to add GetWidgetState
  - [ ] Implement GetWidgetState in `debug-server`
  - [ ] Add widget state message types
- [ ] Write integration tests
  - [ ] Test button click via RPC
  - [ ] Test slider drag via RPC
  - [ ] Test keyboard navigation
  - [ ] Add golden image tests

## Phase 3: Complex Widgets
- [ ] Implement `Knob` widget
  - [ ] Create `gui/src/widgets/knob.rs`
  - [ ] Implement vertical drag interaction
  - [ ] Implement value-to-angle mapping
  - [ ] Implement rendering (arc + indicator)
  - [ ] Implement keyboard fine-tuning
  - [ ] Write unit tests
- [ ] Implement `Textbox` widget
  - [ ] Create `gui/src/widgets/textbox.rs`
  - [ ] Implement text storage and cursor position
  - [ ] Implement text input handling
  - [ ] Implement cursor movement (arrow keys, Home/End)
  - [ ] Implement text selection (Shift+arrows)
  - [ ] Implement copy/paste (clipboard integration)
  - [ ] Implement rendering (text + cursor + selection)
  - [ ] Add placeholder text support
  - [ ] Write unit tests
- [ ] Clipboard integration
  - [ ] Research clipboard crate options
  - [ ] Implement platform-specific clipboard access
  - [ ] Add tests for copy/paste
- [ ] Write integration tests
  - [ ] Test knob rotation via RPC
  - [ ] Test textbox typing via RPC
  - [ ] Test text selection and editing
  - [ ] Add golden image tests

## Phase 4: Polish & Testing
- [ ] Visual polish
  - [ ] Add focus indicators (outline/glow)
  - [ ] Refine color scheme for all widgets
  - [ ] Add hover feedback for all interactive elements
  - [ ] Test at different DPI scales
- [ ] Create demo example
  - [ ] Create `gui/examples/widget_demo.rs`
  - [ ] Layout all widgets in a grid
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

## Open Questions (Review Required)
- [ ] Text rendering: Use existing text_renderer or enhance it?
- [ ] Clipboard: Use `arboard` crate or platform-specific impl?
- [ ] Widget IDs: Formal ID system or index-based?
- [ ] Layout system: Implement now or defer?
- [ ] Color scheme: Hardcoded or theme system?
