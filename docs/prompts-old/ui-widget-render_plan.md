# Widget Framework Completion - Implementation Plan

## Goal

Complete the widget framework by enabling text rendering in widgets, implementing RPC testing infrastructure, and creating a visual demo application.

## User Review Required

> [!IMPORTANT]
> **Design Decision: Widget::render Signature**
> 
> The current `Widget::render()` signature cannot support text rendering because `TextRenderer.draw_text()` requires `VulkanContext` and `FontAtlas` which aren't in the trait.
> 
> **Proposed Solution**: Modify `Widget::render()` to accept additional context:
> ```rust
> fn render(
>     &self,
>     shape_renderer: &mut ShapeRenderer,
>     text_renderer: &mut TextRenderer,
>     vulkan_context: &VulkanContext,
>     font_atlas: &mut FontAtlas,
>     screen_width: u32,
>     screen_height: u32,
> );
> ```
> 
> **Impact**: All existing widgets (Button, Slider, Knob, Textbox) will need signature updates.
> 
> **Alternative**: Keep signature minimal and handle text in Renderer, but this reduces widget encapsulation.

## Proposed Changes

### Part 1: Text Rendering Integration

#### [MODIFY] [gui/src/widgets/mod.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/mod.rs)

**Changes to Widget trait**:
- Update `Widget::render()` signature to include `vulkan_context: &VulkanContext` and `font_atlas: &mut FontAtlas`
- This enables widgets to call `text_renderer.draw_text()` directly

---

#### [MODIFY] [gui/src/widgets/button.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/button.rs)

**Add text rendering**:
- Update [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) signature
- Call `text_renderer.draw_text()` to render button label
- Center text within button bounds

---

#### [MODIFY] [gui/src/widgets/textbox.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/textbox.rs)

**Add text and cursor rendering**:
- Update [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) signature
- Render actual text content
- Render blinking cursor at cursor position
- Render selection highlight if [has_selection()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/textbox.rs#65-69)
- Render placeholder text when empty

---

#### [MODIFY] [gui/src/widgets/container.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/container.rs)

**Add render method**:
- Create `WidgetContainer::render()` method that takes all rendering context
- Iterate over widgets and call their [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) methods
- Pass through VulkanContext and FontAtlas

---

#### [MODIFY] [gui/src/lib.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#L210-L217)

**Integrate widget rendering in GuiContext**:
- Update `GuiContext::render()` to actually render widgets
- Get VulkanContext and FontAtlas from Renderer
- Call `widgets.render()` with full context
- This connects widgets to the actual rendering pipeline

---

### Part 2: RPC Testing Infrastructure

#### [MODIFY] [gui/src/lib.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs)

**Add widget query methods**:
- `GuiContext::get_widget_state(widget_id: u64) -> Result<WidgetStateInfo>`
- `GuiContext::set_widget_value(widget_id: u64, value: f64) -> Result<()>`
- `GuiContext::list_widgets() -> Vec<WidgetStateInfo>`
- Create `WidgetStateInfo` struct to hold widget data for RPC

---

#### [MODIFY] [debug-server/src/lib.rs](file:///Users/mrwilson/Software/rust-vst-2/debug-server/src/lib.rs#L155-L197)

**Implement widget RPC handlers**:
- Add `Arc<Mutex<Option<GuiContext>>>` or widget accessor to [DebugControlImpl](file:///Users/mrwilson/Software/rust-vst-2/debug-server/src/lib.rs#76-79)
- Implement [get_widget_state()](file:///Users/mrwilson/Software/rust-vst-2/debug-server/src/lib.rs#156-170) to query actual widget data
- Implement [set_widget_value()](file:///Users/mrwilson/Software/rust-vst-2/debug-server/src/lib.rs#171-185) to programmatically control widgets
- Implement [list_widgets()](file:///Users/mrwilson/Software/rust-vst-2/debug-server/src/lib.rs#186-196) to return all widget states
- Convert between `gui::WidgetStateInfo` and `debug_control::WidgetState` proto

---

#### [NEW] [gui/tests/widget_rpc_test.rs](file:///Users/mrwilson/Software/rust-vst-2/gui/tests/widget_rpc_test.rs)

**RPC integration tests**:
- Test [GetWidgetState](file:///Users/mrwilson/Software/rust-vst-2/definitions/debug_control.proto#9-11) RPC for each widget type
- Test [SetWidgetValue](file:///Users/mrwilson/Software/rust-vst-2/definitions/debug_control.proto#11-12) RPC to control widgets remotely
- Test [ListWidgets](file:///Users/mrwilson/Software/rust-vst-2/definitions/debug_control.proto#12-13) RPC returns correct widget count
- Verify Button click via RPC
- Verify Slider drag via RPC
- Verify Knob rotation via RPC
- Verify Textbox text editing via RPC

---

### Part 3: Visual Demo Application

#### [NEW] [standalone/examples/widget_showcase.rs](file:///Users/mrwilson/Software/rust-vst-2/standalone/examples/widget_showcase.rs)

**Interactive widget demo**:
- Create window with Renderer and GuiContext
- Add all 4 widget types with labels
- Use GridLayout or FlexLayout for organization
- Wire up event loop to process [handle_event()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/widgets/slider.rs#93-163)
- Render widgets every frame via `GuiContext::render()`
- Add console logging for widget value changes
- Include instructions overlay

**Features**:
- Button: Click to increment counter
- Slider: Drag to adjust value, display current value
- Knob: Vertical drag to adjust, show angle
- Textbox: Type text, test copy/paste

---

## Verification Plan

### Manual Testing

1. **Run widget showcase**: `cargo run --example widget_showcase`
   - Verify all widgets render with text
   - Test mouse and keyboard interactions
   - Verify focus navigation (Tab key)
   - Test clipboard in textbox (Cmd+C/V/X)

2. **RPC Testing**: Run standalone with RPC client
   - Query widget states
   - Set widget values remotely
   - List all widgets

### Automated Tests

- Run `cargo test -p gui` - all widget unit tests pass
- Run `cargo test -p gui --test widget_demo` - demo tests pass
- Run `cargo test -p gui --test widget_rpc_test` - RPC tests pass
- Verify no warnings: `cargo build --workspace`

---

## Breaking Changes

> [!WARNING]
> **Widget::render() signature change**
> 
> All widgets will need to update their [render()](file:///Users/mrwilson/Software/rust-vst-2/gui/src/lib.rs#133-138) implementation. This is a one-time breaking change to enable text rendering.

---

## Timeline Estimate

- Part 1 (Text Rendering): ~2 hours
- Part 2 (RPC Infrastructure): ~1.5 hours  
- Part 3 (Visual Demo): ~1 hour
- Testing & Polish: ~30 minutes

**Total**: ~5 hours
