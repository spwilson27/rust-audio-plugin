# Phase 2.5: Standalone Mode - Complete

## Summary

Successfully implemented standalone mode with native macOS window creation, NSApplication setup, and event loop integration. The standalone binary now opens a real window titled "splug - Standalone" that can be interacted with.

## Changes Made

### 1. Standalone Binary ([standalone/src/main.rs](file:///Users/mrwilson/Software/rust-vst/standalone/src/main.rs))

**NSApplication Initialization:**
```rust
let ns_app_class = AnyClass::get("NSApplication")
    .expect("NSApplication class not found");
let app: Retained<AnyObject> = msg_send_id![ns_app_class, sharedApplication];

// Set activation policy to show in Dock
let policy: i64 = 0; // NSApplicationActivationPolicyRegular
let _: () = objc2::msg_send![&*app, setActivationPolicy: policy];
```

**NSWindow Creation:**
```rust
let window_rect = objc2_foundation::NSRect {
    origin: NSPoint { x: 100.0, y: 100.0 },
    size: NSSize { width: 800.0, height: 600.0 },
};

let style_mask: u64 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);
// Titled | Closable | Miniaturizable | Resizable

let window: Retained<AnyObject> = msg_send_id![
    msg_send_id![ns_window_class, alloc],
    initWithContentRect: window_rect
    styleMask: style_mask
    backing: 2u64  // NSBackingStoreBuffered
    defer: false
];
```

**GUI Context Integration:**
```rust
// Get content view from NSWindow
let content_view: *mut AnyObject = objc2::msg_send![&*window, contentView];

// Create PAL window attached to content view
let mut gui_ctx = gui::GuiContext::attach(
    content_view as *mut c_void,
    800,
    600,
)?;

// Set up event logging
if let Some(window) = gui_ctx.get_window_mut() {
    window.set_event_callback(|event| {
        println!("Event: {:?}", event);
    });
}
```

**Event Loop:**
```rust
// Show window
let _: () = objc2::msg_send![&*window, makeKeyAndOrderFront: null()];

// Activate app
let _: () = objc2::msg_send![&*app, activateIgnoringOtherApps: true];

// Run event loop (blocks until window closes)
let _: () = objc2::msg_send![&*app, run];
```

### 2. AppKit Framework Linking ([standalone/build.rs](file:///Users/mrwilson/Software/rust-vst/standalone/build.rs))

Created build script to link AppKit framework:

```rust
fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AppKit");
    }
}
```

**Why This Was Needed:**
- `AnyClass::get("NSApplication")` requires AppKit to be linked at runtime
- Without the framework link, the Objective-C runtime can't find NSApplication/NSWindow classes
- The `println!("cargo:rustc-link-lib...")` directive tells the linker to include AppKit

### 3. Dependencies Added ([standalone/Cargo.toml](file:///Users/mrwilson/Software/rust-vst/standalone/Cargo.toml))

```toml
[dependencies]
gui = { path = "../gui" }
pal = { path = "../pal" }

[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
objc2-foundation = { version = "0.2", features = ["NSString"] }
objc2-app-kit = { version = "0.2", features = ["NSApplication", "NSWindow"] }
```

### 4. GUI Layer Helper ([gui/src/lib.rs](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs))

Added method to access underlying PAL window:

```rust
#[cfg(target_os = "macos")]
pub fn get_window_mut(&mut self) -> Option<&mut pal::MacOSWindow> {
    Some(&mut self.window)
}
```

This allows standalone to set event callbacks on the PAL window.

### 5. PAL UIEvent Export ([pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs))

Made `UIEvent` public so GUI layer can use it:

```rust
pub enum UIEvent {
    MouseDown { x: f64, y: f64, button: u32 },
    MouseUp { x: f64, y: f64, button: u32 },
    MouseMove { x: f64, y: f64 },
    KeyDown { keycode: u16 },
    KeyUp { keycode: u16 },
}
```

## Build Verification

### Clean Build
```
cargo build --workspace
```
**Result:** SUCCESS - Zero warnings

### Tests
```
cargo test --workspace
```
**Results:** All 5 tests passing

### Standalone Window
```
cargo run --bin standalone
```

**Output:**
```
splug standalone host v0.1.0
Running with GUI...
Initializing macOS window...
Creating GUI context...
Setting up event logging...
Opening window...

Window opened!
Close the window to exit.
```

**Behavior:**
- Window appears at (100, 100) with size 800x600
- Title shows "splug - Standalone"
- Can be resized, minimized, closed
- Appears in Dock with proper app activation
- Event loop runs until window closes
- Clean shutdown when closed

## Current Architecture

```
standalone/src/main.rs     -> Creates NSApplication + NSWindow
    ↓
gui::GuiContext::attach()  -> Creates PAL window from content view
    ↓
pal::MacOSWindow::attach() -> Creates NSView as child
    ↓
Event callback             -> Logs events to stdout (ready for Phase 2.3)
```

## What Works

1. **Top-Level Window Creation:** NSApplication and NSWindow created successfully
2. **PAL Integration:** Content view extracted and passed to GuiContext
3. **Event Loop:** NSApplication.run() handles all window events
4. **AppKit Linking:** build.rs correctly links framework
5. **Cleanup:** Window closes cleanly, app terminates properly
6. **Event Callback API:** Ready to log events (actual delivery pending Phase 2.3)

## What's Pending

**Phase 2.3: NSView Subclass for Event Delivery**
- Currently events aren't being delivered to our callback
- Need to create custom NSView subclass with objc2::declare::ClassBuilder
- Override mouseDown:, mouseUp:, keyDown:, etc.
- Store Rust callback in associated object
- Call Rust function from Objective-C method implementations

**Phase 2.4: CAMetalLayer**
- Add Metal-compatible layer for Vulkan rendering
- Set up for Phase 3 Vulkan swapchain

## Technical Challenges Solved

**Challenge 1: NSApplication Not Found**
- Error: `class NSApplication could not be found`
- Root cause: AppKit framework not linked
- **Solution:** Created build.rs with `cargo:rustc-link-lib=framework=AppKit`

**Challenge 2: Deprecated Class Type**
- Warning: `Class` renamed to `AnyClass`
- **Solution:** Replaced all `Class::get()` with `AnyClass::get()`

**Challenge 3: GUI Context Access**
- Need to set callback on PAL window
- **Solution:** Added [get_window_mut()](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs#140-145) helper in GuiContext

## Files Modified

| File | LOC | Description |
|------|-----|-------------|
| standalone/src/main.rs | +90 | NSApplication/NSWindow creation, event loop |
| standalone/build.rs | +9 | AppKit framework linking |
| standalone/Cargo.toml | +8 | objc2 dependencies |
| gui/src/lib.rs | +11 | get_window_mut() helper |
| pal/src/macos.rs | +1 | Made UIEvent public |

## Next Steps

**Option 1: Phase 3 - Vulkan Graphics** (RECOMMENDED)
- Initialize Vulkan context
- Create swapchain
- Render to the window we just created
- Visual feedback for development

**Option 2: Phase 2.3 - NSView Subclass**
- Complete event handling
- Test mouse/keyboard with visual feedback
- Requires 2-4 hours of objc2 runtime work

**Option 3: Phase 2.4 - CAMetalLayer**
- Add Metal layer to NSView
- Prepare for Vulkan surface creation
- Natural prerequisite for Phase 3

## Status

**Phase 2.5: COMPLETE**
- Standalone window opens ✓
- NSApplication integration ✓  
- Event loop working ✓
- Clean shutdown ✓
- Zero warnings ✓
- All tests passing ✓

Ready to proceed to Phase 3 (Vulkan Graphics) or complete Phase 2.3 (NSView subclass for events).
