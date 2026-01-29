# Phase 2.3: Event Delivery System - Implementation Plan

## Goal

Implement actual event delivery from macOS to Rust by creating a custom NSView subclass and an event middleware layer that can accept events from both hardware (mouse/keyboard) and the debug RPC server.

## Architecture Overview

```
Hardware Events (NSView)  ─┐
                          ├──> EventRouter ──> Callback
RPC Debug Server         ─┘
```

**Event Flow:**
1. NSView subclass receives OS events (mouseDown:, keyDown:, etc.)
2. Events converted to platform-agnostic `UIEvent` enum
3. Routed through `EventRouter` middleware
4. Delivered to registered callback
5. RPC server can inject events into same router

## Proposed Changes

### 1. Event Middleware Layer (`pal/src/event_router.rs`)

**Purpose:** Abstract event source (hardware vs RPC) from event consumers.

**API:**
```rust
pub struct EventRouter {
    callback: Option<Box<dyn FnMut(UIEvent) + Send>>,
}

impl EventRouter {
    pub fn new() -> Self;
    pub fn set_callback<F>(&mut self, callback: F) where F: FnMut(UIEvent) + Send + 'static;
    pub fn route_event(&mut self, event: UIEvent);
    
    // For RPC injection (Phase 5)
    pub fn inject_event(&mut self, event: UIEvent) {
        self.route_event(event);
    }
}
```

**Design Decision:**
- Use `Box<dyn FnMut>` for now (GUI thread only, no real-time constraints)
- Phase 4 will add `rtrb` queue for audio thread communication
- Thread-safe via main thread guarantee

### 2. NSView Subclass ([pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs))

**Challenge:** Create Objective-C class at runtime using objc2.

**Approach:**
```rust
use objc2::declare::ClassBuilder;
use objc2::runtime::{AnyObject, Sel};
use objc2::{sel, msg_send};

// Create custom NSView subclass
fn create_rust_view_class() -> &'static AnyClass {
    let superclass = AnyClass::get("NSView").unwrap();
    let mut builder = ClassBuilder::new("RustPluginView", superclass).unwrap();
    
    // Override acceptsFirstResponder to return YES
    unsafe {
        builder.add_method(
            sel!(acceptsFirstResponder),
            accepts_first_responder as extern "C" fn(&AnyObject, Sel) -> bool
        );
        
        // Override mouse events
        builder.add_method(
            sel!(mouseDown:),
            mouse_down as extern "C" fn(&AnyObject, Sel, *mut AnyObject)
        );
        
        // ... more event methods
    }
    
    builder.register()
}

// C-compatible event handlers
extern "C" fn accepts_first_responder(_self: &AnyObject, _sel: Sel) -> bool {
    true
}

extern "C" fn mouse_down(this: &AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        // Extract event data
        let location: NSPoint = msg_send![event, locationInWindow];
        let button: i64 = msg_send![event, buttonNumber];
        
        // Get EventRouter from associated object
        if let Some(router) = get_event_router(this) {
            router.route_event(UIEvent::MouseDown {
                x: location.x,
                y: location.y,
                button: button as u32,
            });
        }
    }
}
```

**Storing EventRouter Reference:**
- Use `objc2::rc::set_associated_object` to store pointer to EventRouter
- Retrieve in event handlers
- Critical: Ensure lifetime safety (EventRouter lives as long as view)

**Event Methods to Override:**
- `acceptsFirstResponder` → return YES
- `mouseDown:` → left click
- `mouseUp:` → left release
- `rightMouseDown:` → right click
- `rightMouseUp:` → right release
- `mouseMoved:` → cursor movement
- `mouseDragged:` → drag with button down
- `keyDown:` → key press
- `keyUp:` → key release

### 3. MacOSWindow Integration

**Update [MacOSWindow](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#58-67):**
```rust
pub struct MacOSWindow {
    view: Retained<AnyObject>,
    width: u32,
    height: u32,
    scale_factor: f64,
    event_router: EventRouter,  // Changed from callback
}

impl MacOSWindow {
    pub fn attach(parent: *mut c_void) -> Result<Self> {
        // Create instance of RustPluginView instead of NSView
        let view_class = create_rust_view_class();
        let view: Retained<AnyObject> = msg_send_id![view_class, new];
        
        // ... existing setup code ...
        
        let mut window = MacOSWindow {
            view,
            width,
            height,
            scale_factor,
            event_router: EventRouter::new(),
        };
        
        // Store pointer to event_router in view's associated objects
        unsafe {
            set_associated_object(
                &*window.view,
                EVENT_ROUTER_KEY,
                &window.event_router as *const _ as *mut c_void,
            );
        }
        
        Ok(window)
    }
    
    pub fn set_event_callback<F>(&mut self, callback: F)
    where F: FnMut(UIEvent) + Send + 'static
    {
        self.event_router.set_callback(callback);
    }
}
```

### 4. Coordinate Transformation

**macOS to Vulkan Coordinates:**
```rust
fn transform_coordinates(cocoa_point: NSPoint, view_height: f64) -> (f64, f64) {
    // Cocoa: bottom-left origin
    // Vulkan: top-left origin
    let vulkan_y = view_height - cocoa_point.y;
    (cocoa_point.x, vulkan_y)
}
```

Apply in event handlers before routing.

### 5. Testing Strategy

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_event_router() {
        let mut router = EventRouter::new();
        let mut received = None;
        
        router.set_callback(|event| {
            received = Some(event);
        });
        
        router.route_event(UIEvent::MouseDown { x: 100.0, y: 200.0, button: 0 });
        
        assert!(matches!(received, Some(UIEvent::MouseDown { .. })));
    }
}
```

**Manual Tests:**
1. Run standalone
2. Click in window → see stdout log with coordinates
3. Type keys → see keystroke logs
4. Drag mouse → see movement events
5. Close window → clean shutdown

## Implementation Steps

1. **Create `event_router.rs`** - Middleware abstraction
2. **Update `UIEvent` enum** - Add all event types
3. **Create NSView subclass** - Runtime class with objc2
4. **Implement event handlers** - mouseDown, keyDown, etc.
5. **Add associated object storage** - EventRouter pointer
6. **Update MacOSWindow** - Use new view class
7. **Test event delivery** - Standalone logging
8. **Documentation** - Comment complex objc2 usage

## Technical Challenges

### Challenge 1: Associated Objects Lifetime

**Problem:** EventRouter is owned by MacOSWindow, but NSView needs to access it.

**Solution:**
- Store raw pointer in associated object
- Guarantee: MacOSWindow lives as long as view (Drop removes view)
- Access: Dereference pointer in event handlers

**Safety:**
```rust
// In attach()
let router_ptr = &self.event_router as *const EventRouter as *mut c_void;
set_associated_object(&*self.view, KEY, router_ptr);

// In event handler
let router_ptr = get_associated_object(this, KEY) as *mut EventRouter;
if !router_ptr.is_null() {
    let router = &mut *router_ptr;
    router.route_event(event);
}
```

### Challenge 2: NSEvent Parsing

Extract data from NSEvent*:
```rust
let location: NSPoint = msg_send![event, locationInWindow];
let button_number: i64 = msg_send![event, buttonNumber];
let key_code: u16 = msg_send![event, keyCode];
let modifiers: u64 = msg_send![event, modifierFlags];
```

Parse modifier flags:
```rust
const NSEventModifierFlagShift: u64 = 1 << 17;
const NSEventModifierFlagControl: u64 = 1 << 18;
const NSEventModifierFlagOption: u64 = 1 << 19;
const NSEventModifierFlagCommand: u64 = 1 << 20;
```

### Challenge 3: ClassBuilder Usage

**objc2 0.5 API:**
```rust
use objc2::declare::ClassBuilder;

let mut builder = ClassBuilder::new("RustPluginView", superclass)?;

// Add method with C function pointer
builder.add_method(
    sel!(acceptsFirstResponder),
    accepts_first_responder as extern "C" fn(&AnyObject, Sel) -> bool
);

// Register returns &'static AnyClass
let class = builder.register();
```

**Method Signature Rules:**
- First param: `&AnyObject` (self)
- Second param: `Sel` (selector)
- Remaining params: method arguments
- Return type matches Objective-C type

## Files to Modify

| File | Changes | LOC |
|------|---------|-----|
| `pal/src/event_router.rs` | New file - event middleware | +80 |
| [pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs) | NSView subclass, event handlers | +200 |
| [pal/src/lib.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/lib.rs) | Export EventRouter, update UIEvent | +20 |
| [standalone/src/main.rs](file:///Users/mrwilson/Software/rust-vst/standalone/src/main.rs) | Enhanced event logging | +10 |

## Success Criteria

- [x] EventRouter middleware compiles and tests pass
- [x] NSView subclass creates successfully
- [x] Mouse clicks logged to stdout with correct coordinates
- [x] Keyboard events logged with key codes
- [x] Mouse movement tracked
- [x] Coordinate transformation correct (top-left origin)
- [x] No crashes or memory leaks
- [x] RPC injection point ready (Phase 5)

## Next Steps After Completion

With Phase 2.3 complete, we can:
1. **Phase 3:** Vulkan rendering (have window + events)
2. **Phase 2.4:** CVDisplayLink render loop
3. **Phase 5:** RPC server using EventRouter.inject_event()

---

**Estimated Time:** 4-6 hours
**Complexity:** High (objc2 runtime, FFI, unsafe code)
**Priority:** High (enables all future UI development)
