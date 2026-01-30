# Phase 2.3: Event Delivery - NSView Subclass Implementation

## Objective
Implemented a custom NSView subclass (`RustPluginView`) using objc2's runtime capabilities to capture native macOS events (mouse and keyboard) and route them through the [EventRouter](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs#14-17) middleware. This enables the application to receive and process user input, forming a crucial part of the Platform Abstraction Layer (PAL).

## Changes Made

### 1. EventRouter Middleware ([pal/src/event_router.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs))
Created event routing middleware to decouple event sources from consumers:
- **EventRouter struct** with mutable callback storage
- **set_callback()** - Register event handlers  
- **route_event()** - Deliver events to registered callback
- **inject_event()** - Future RPC server integration
- Tests passing without Send bound issues

### 2. UIEvent Enum Reorg ([pal/src/lib.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/lib.rs))
- Moved `UIEvent` to crate root for cross-module access
- Variants: `MouseDown`, `MouseUp`, `MouseMove`, `KeyDown`, `KeyUp`

### 3. Custom NSView Subclass ([pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs))

#### RustPluginView Class Creation
Implemented runtime class creation using `objc2::declare::ClassBuilder`:
```rust
fn get_rust_view_class() -> &'static AnyClass {
    // One-time registration using std::sync::Once
    // Creates "RustPluginView" as NSView subclass
    // Registers 9 methods total
}
```

#### Event Handlers Implemented (8 total)
All using raw pointers (`*mut AnyObject`) for objc2 compatibility:

**Mouse Events:**
- [mouse_down](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#201-211) - Left button press
- [mouse_up](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#212-222) - Left button release  
- [right_mouse_down](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#223-233) - Right button press
- [right_mouse_up](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#234-244) - Right button release
- [mouse_moved](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#245-267) - Cursor movement
- [mouse_dragged](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#268-273) - Drag operation

**Keyboard Events:**
- [key_down](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#274-288) - Key press
- [key_up](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#289-303) - Key release

**Special Override:**
- [accepts_first_responder](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#139-143) - Returns `objc2::runtime::Bool(YES)` to enable keyboard events

#### Coordinate Transformation
Implemented Cocoa→Vulkan coordinate conversion:
```rust
// Cocoa: bottom-left origin
// Vulkan: top-left origin  
let vulkan_y = view_height - cocoa_y;
```

#### Associated Object Storage
Event handlers access [EventRouter](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs#14-17) via associated objects:
```rust
// Store EventRouter pointer in view's associated objects
object_setInstanceVariable(
    view_ptr as *mut objc2::runtime::AnyObject as *mut objc2::ffi::objc_object,
    "RustEventRouterPointer",
    router_ptr,
);

// Retrieve in event handlers
unsafe fn get_event_router(view: *mut AnyObject) -> Option<&'static mut EventRouter>
```

### 4. MacOSWindow Integration
- Replaced plain `NSView` with `RustPluginView` in [attach()](file:///Users/mrwilson/Software/rust-vst/pal/src/win32.rs#26-30)
- Changed [event_callback](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#442-460) field to `event_router: EventRouter`
- Store [EventRouter](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs#14-17) pointer after window creation
- Updated [set_event_callback()](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#442-460) to delegate to router
- Updated [trigger_event()](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#461-466) for testing

### 5. Standalone App Logging
- Added `window.set_event_callback` in [standalone/src/main.rs](file:///Users/mrwilson/Software/rust-vst/standalone/src/main.rs)
- Logs received events to console

### 6. Critical Fix: EventRouter Stability & Storage
- **Issue 1:** [EventRouter](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs#14-17) structure moved in memory, invalidating pointers.
  - **Fix:** Boxed the [EventRouter](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs#14-17) (`Box<EventRouter>`) to ensure stable heap address.
- **Issue 2:** `object_setInstanceVariable` failed because Ivar slot didn't exist.
  - **Fix:** Added `builder.add_ivar::<*mut c_void>(EVENT_ROUTER_KEY)` in ClassBuilder.
- **Issue 3:** Incorrect usage of `object_setInstanceVariable` for reading.
  - **Fix:** Switched to `object_getInstanceVariable` with correct pointer-to-pointer signature (`*mut *const c_void`).

```rust
// Class Registration
builder.add_ivar::<*mut c_void>(EVENT_ROUTER_KEY);

// Storage (in attach)
object_setInstanceVariable(view, key, router_ptr);

// Retrieval (in event handlers)
let mut out_ptr: *const c_void = std::ptr::null();
object_getInstanceVariable(view, key, &mut out_ptr);
```

## objc2 API Debugging & Crash Fix

### Issues Resolved:
1. **Function Signatures** - Required raw pointers (`*mut AnyObject`) instead of references (`&AnyObject`)  
2. **Bool Encoding (CRASH FIX)** - `acceptsFirstResponder` returns `objc2::runtime::Bool` (encoding 'B') instead of `c_char` (encoding 'c'). This resolved the runtime panic.
3. **Associated Objects** - Used deprecated `object_setInstanceVariable` API (warnings acceptable for now). Fixed missing type info for casts.
4. **Type Casts** - Required intermediate casts: `view as *mut objc2::runtime::AnyObject as *mut objc2::ffi::objc_object`.
5. **msg_send_id! Macro** - Simplified to `msg_send_id![class, new]` instead of `alloc, init`.

## Build Status

**Final:**  
- Compiles Successfully  
- 7 Warnings (deprecation, unused imports)
- 0 Errors

**Warnings:**
- Deprecated `object_setInstanceVariable` (acceptable until objc2 provides modern alternative)
- Unused `msg_send_id` import  
- Unnecessary [mut](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs#146-150) in [get_event_router](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs#144-169)

## Lines of Code
- [pal/src/macos.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/macos.rs): ~470 lines (+180)
  - NSView subclass: ~150 lines
  - Event handlers: ~90 lines
  - Integration: ~20 lines
- [pal/src/event_router.rs](file:///Users/mrwilson/Software/rust-vst/pal/src/event_router.rs): ~130 lines (new file)

## Testing Status
- [x] EventRouter unit tests passing
- [x] pal package builds
- [x] standalone compiles and runs
- [x] Manual verification: Window opens without panic, events logic added.

## Next Steps
1. Verify mouse clicks and key presses generate log output in standalone console.
2. Consider replacing deprecated `object_setInstanceVariable` with modern alternative.
3. Clean up warnings (remove unused imports, fix [mut](file:///Users/mrwilson/Software/rust-vst/gui/src/lib.rs#146-150)).
