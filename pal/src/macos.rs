//! macOS Platform Abstraction Layer
//!
//! Implements NativeWindow using Cocoa/AppKit via objc2.
//!
//! ## Key Concepts
//!
//! - NSView subclass with CAMetalLayer backing for Vulkan/Metal compatibility
//! - CVDisplayLink for 60 FPS render loop (DAW owns event loop)
//! - viewDidMoveToWindow for visibility detection
//! - Coordinate system translation (Cocoa bottom-left → Vulkan top-left)

use anyhow::{Context, Result};
use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
use std::ffi::c_void;

// CVDisplayLink FFI declarations (CoreVideo framework)
type CVDisplayLinkRef = *mut c_void;
type CVReturn = i32;
type CVOptionFlags = u64;

#[repr(C)]
struct CVTimeStamp {
    _data: [u8; 96], // Opaque structure
}

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVDisplayLinkCreateWithActiveCGDisplays(display_link: *mut CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkSetOutputCallback(
        display_link: CVDisplayLinkRef,
        callback: extern "C" fn(
            CVDisplayLinkRef,
            *const CVTimeStamp,
            *const CVTimeStamp,
            CVOptionFlags,
            *mut CVOptionFlags,
            *mut c_void,
        ) -> CVReturn,
        user_info: *mut c_void,
    ) -> CVReturn;
    fn CVDisplayLinkStart(display_link: CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkStop(display_link: CVDisplayLinkRef) -> CVReturn;
    fn CVDisplayLinkRelease(display_link: CVDisplayLinkRef);
}

const K_CV_RETURN_SUCCESS: CVReturn = 0;

use objc2::encode::{Encode, Encoding};
// use objc2::msg_send_id;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::MainThreadMarker;

// NSRect/NSPoint/NSSize definitions matching macOS ABI
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct NSPoint {
    x: f64,
    y: f64,
}

unsafe impl Encode for NSPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct NSSize {
    width: f64,
    height: f64,
}

unsafe impl Encode for NSSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct NSRect {
    origin: NSPoint,
    size: NSSize,
}

unsafe impl Encode for NSRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[NSPoint::ENCODING, NSSize::ENCODING]);
}

// ============================================================================
// NSView Subclass Implementation
// ============================================================================

use objc2::declare::ClassBuilder;
use objc2::runtime::{AnyClass, Sel};
use objc2::sel;
// use std::sync::Once;

/// Key for storing EventRouter pointer in NSView associated objects
static EVENT_ROUTER_KEY: &str = "RustEventRouterPointer";

/// Create the RustPluginView NSView subclass at runtime
///
/// This function uses objc2's ClassBuilder to create an Objective-C class
/// that inherits from NSView and overrides event handling methods.
fn get_rust_view_class() -> &'static AnyClass {
    static REGISTER_CLASS: std::sync::Once = std::sync::Once::new();
    static mut CLASS: *const AnyClass = std::ptr::null();

    unsafe {
        REGISTER_CLASS.call_once(|| {
            let ns_view_class = AnyClass::get("NSView").expect("NSView class not found");
            let mut builder = ClassBuilder::new("RustPluginView", ns_view_class).unwrap();

            // Add Ivar to store the EventRouter pointer
            builder.add_ivar::<*mut c_void>(EVENT_ROUTER_KEY);

            // Override acceptsFirstResponder to return YES so we receive keyboard events
            builder.add_method(
                sel!(acceptsFirstResponder),
                accepts_first_responder
                    as extern "C" fn(*mut AnyObject, Sel) -> objc2::runtime::Bool,
            );

            // Override acceptsFirstMouse to return YES so we receive mouse events when clicking an unfocused window
            builder.add_method(
                sel!(acceptsFirstMouse:),
                accepts_first_mouse
                    as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject) -> objc2::runtime::Bool,
            );

            // Mouse event handlers
            builder.add_method(
                sel!(mouseDown:),
                mouse_down as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            builder.add_method(
                sel!(mouseUp:),
                mouse_up as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            builder.add_method(
                sel!(rightMouseDown:),
                right_mouse_down as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            builder.add_method(
                sel!(rightMouseUp:),
                right_mouse_up as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            builder.add_method(
                sel!(mouseMoved:),
                mouse_moved as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            builder.add_method(
                sel!(mouseDragged:),
                mouse_dragged as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );

            // Keyboard event handlers
            builder.add_method(
                sel!(keyDown:),
                key_down as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );
            builder.add_method(
                sel!(keyUp:),
                key_up as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );

            // Text Input
            builder.add_method(
                sel!(insertText:),
                insert_text as extern "C" fn(*mut AnyObject, Sel, *mut AnyObject),
            );

            // Command Selector Handler (to suppress beeps)
            builder.add_method(
                sel!(doCommandBySelector:),
                do_command_by_selector as extern "C" fn(*mut AnyObject, Sel, Sel),
            );

            let cls = builder.register();
            CLASS = cls;
        });
        &*CLASS
    }
}

// ============================================================================
// Event Handler Implementations
// ============================================================================

/// Accept first responder to receive keyboard events
extern "C" fn accepts_first_responder(_this: *mut AnyObject, _sel: Sel) -> objc2::runtime::Bool {
    objc2::runtime::Bool::YES
}

/// Accept first mouse to receive mouse events when window is not key
extern "C" fn accepts_first_mouse(
    _this: *mut AnyObject,
    _sel: Sel,
    _event: *mut AnyObject,
) -> objc2::runtime::Bool {
    objc2::runtime::Bool::YES
}

/// Helper to get EventRouter from associated object
unsafe fn get_event_router(view: *mut AnyObject) -> Option<&'static mut crate::EventRouter> {
    #[allow(deprecated)]
    use objc2::ffi::object_getInstanceVariable; // Correct import: GET
    use std::ffi::CString;

    if view.is_null() {
        return None;
    }

    let key = CString::new(EVENT_ROUTER_KEY).unwrap();
    let mut out_ptr: *const c_void = std::ptr::null();

    // Get the instance variable
    #[allow(deprecated)]
    object_getInstanceVariable(
        view as *mut _ as *mut objc2::ffi::objc_object,
        key.as_ptr(),
        &mut out_ptr,
    );

    if out_ptr.is_null() {
        None
    } else {
        Some(&mut *(out_ptr as *mut crate::EventRouter))
    }
}

/// Helper to extract mouse event data and transform coordinates
unsafe fn handle_mouse_event(
    view: *mut AnyObject,
    event: *mut AnyObject,
    button: u32,
    event_type: fn(f64, f64, u32) -> crate::UIEvent,
) {
    if event.is_null() || view.is_null() {
        return;
    }

    // Get mouse location in window coordinates
    let location: NSPoint = objc2::msg_send![event, locationInWindow];

    // Convert to view coordinates
    let view_location: NSPoint =
        objc2::msg_send![view, convertPoint: location fromView: std::ptr::null::<AnyObject>()];

    // Get view height for coordinate transformation
    let bounds: NSRect = objc2::msg_send![view, bounds];
    let view_height = bounds.size.height;

    // Transform from Cocoa coordinates (bottom-left origin) to Vulkan (top-left origin)
    let vulkan_y = view_height - view_location.y;

    // Route event through EventRouter
    if let Some(router) = get_event_router(view) {
        router.route_event(event_type(view_location.x, vulkan_y, button));
    }
}

/// Mouse down handler (left button)
extern "C" fn mouse_down(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        handle_mouse_event(this, event, 0, |x, y, b| crate::UIEvent::MouseDown {
            x,
            y,
            button: b,
        });
    }
}

/// Mouse up handler (left button)
extern "C" fn mouse_up(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        handle_mouse_event(this, event, 0, |x, y, b| crate::UIEvent::MouseUp {
            x,
            y,
            button: b,
        });
    }
}

/// Right mouse down handler
extern "C" fn right_mouse_down(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        handle_mouse_event(this, event, 1, |x, y, b| crate::UIEvent::MouseDown {
            x,
            y,
            button: b,
        });
    }
}

/// Right mouse up handler
extern "C" fn right_mouse_up(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        handle_mouse_event(this, event, 1, |x, y, b| crate::UIEvent::MouseUp {
            x,
            y,
            button: b,
        });
    }
}

/// Mouse moved handler
extern "C" fn mouse_moved(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        if event.is_null() {
            return;
        }

        let location: NSPoint = objc2::msg_send![event, locationInWindow];
        let view_location: NSPoint =
            objc2::msg_send![this, convertPoint: location fromView: std::ptr::null::<AnyObject>()];
        let bounds: NSRect = objc2::msg_send![this, bounds];
        let view_height = bounds.size.height;
        let vulkan_y = view_height - view_location.y;

        if let Some(router) = get_event_router(this) {
            router.route_event(crate::UIEvent::MouseMove {
                x: view_location.x,
                y: vulkan_y,
            });
        }
    }
}

/// Mouse dragged handler
extern "C" fn mouse_dragged(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    // Treat drag as move for now
    mouse_moved(this, _sel, event);
}

/// Key down handler
extern "C" fn key_down(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        if event.is_null() {
            return;
        }

        let keycode: u16 = objc2::msg_send![event, keyCode];

        // 1. Pass key event to system for text interpretation
        let ns_array_class = AnyClass::get("NSArray").unwrap();
        let events: Retained<AnyObject> =
            objc2::msg_send_id![ns_array_class, arrayWithObject: event];
        let _: () = objc2::msg_send![this, interpretKeyEvents: &*events];

        // Helper to extract modifiers
        let flags: u64 = objc2::msg_send![event, modifierFlags];
        let mut modifiers = crate::Modifiers::empty();

        // NSEventModifierFlagShift = 1 << 17
        if (flags & (1 << 17)) != 0 {
            modifiers.insert(crate::Modifiers::SHIFT);
        }
        // NSEventModifierFlagControl = 1 << 18
        if (flags & (1 << 18)) != 0 {
            modifiers.insert(crate::Modifiers::CTRL);
        }
        // NSEventModifierFlagOption = 1 << 19
        if (flags & (1 << 19)) != 0 {
            modifiers.insert(crate::Modifiers::ALT);
        }
        // NSEventModifierFlagCommand = 1 << 20
        if (flags & (1 << 20)) != 0 {
            modifiers.insert(crate::Modifiers::META);
        }

        // 2. Also dispatch raw KeyDown for navigation keys (Arrows, Esc, etc.)
        if let Some(router) = get_event_router(this) {
            router.route_event(crate::UIEvent::KeyDown {
                keycode: keycode.into(),
                modifiers,
            });
        }
    }
}

/// Insert text handler (called by interpretKeyEvents)
extern "C" fn insert_text(this: *mut AnyObject, _sel: Sel, object: *mut AnyObject) {
    unsafe {
        // object can be NSString or NSAttributedString
        // We only care about the string content
        let string_obj: *mut AnyObject = if objc2::msg_send![object, isKindOfClass: AnyClass::get("NSAttributedString").unwrap()]
        {
            objc2::msg_send![object, string]
        } else {
            object
        };

        // Convert NSString to Rust String
        let utf8_string: *const std::ffi::c_char = objc2::msg_send![string_obj, UTF8String];
        if !utf8_string.is_null() {
            let limit = 1024; // Limit length for safety
            let mut bytes = Vec::new();
            for i in 0..limit {
                let b = *utf8_string.add(i);
                if b == 0 {
                    break;
                }
                bytes.push(b as u8);
            }

            if let Ok(text) = String::from_utf8(bytes) {
                if !text.is_empty() {
                    if let Some(router) = get_event_router(this) {
                        router.route_event(crate::UIEvent::TextInput(text));
                    }
                }
            }
        }
    }
}

/// Handle selectors generated by interpretKeyEvents
extern "C" fn do_command_by_selector(_this: *mut AnyObject, _sel: Sel, _selector: Sel) {
    // By implementing this and NOT calling super, we claim to handle the selector.
    // This prevents NSResponder from beeping (the default behavior for unhandled commands).
    // The actual logic is handled by our separate `key_down` handler which dispatches
    // `UIEvent::KeyDown`, or `insert_text` for typing.

    // We can inspect `selector` if we want to be selective, but for a raw view
    // that handles everything via custom events, consuming all commands is usually safe.
    // Common commands likely to cause beeps if unhandled:
    // - deleteBackward:
    // - deleteForward:
    // - moveLeft:, moveRight:, moveUp:, moveDown:
    // - selectAll:
    // - insertNewline:

    let _: () = (); // Do nothing
}

/// Key up handler
extern "C" fn key_up(this: *mut AnyObject, _sel: Sel, event: *mut AnyObject) {
    unsafe {
        if event.is_null() {
            return;
        }

        let keycode: u16 = objc2::msg_send![event, keyCode];

        let flags: u64 = objc2::msg_send![event, modifierFlags];
        let mut modifiers = crate::Modifiers::empty();

        if (flags & (1 << 17)) != 0 {
            modifiers.insert(crate::Modifiers::SHIFT);
        }
        if (flags & (1 << 18)) != 0 {
            modifiers.insert(crate::Modifiers::CTRL);
        }
        if (flags & (1 << 19)) != 0 {
            modifiers.insert(crate::Modifiers::ALT);
        }
        if (flags & (1 << 20)) != 0 {
            modifiers.insert(crate::Modifiers::META);
        }

        if let Some(router) = get_event_router(this) {
            router.route_event(crate::UIEvent::KeyUp {
                keycode: keycode.into(),
                modifiers,
            });
        }
    }
}

// ============================================================================
// MacOSWindow Implementation
// ============================================================================

/// macOS window implementation using NSView
pub struct MacOSWindow {
    view: Retained<AnyObject>,
    width: u32,
    height: u32,
    scale_factor: f64,
    // EventRouter lives here and pointer is stored in view's associated objects
    // Must be Boxed to ensure stable address when MacOSWindow moves
    event_router: Box<crate::EventRouter>,
    // Optional: Keep reference to window if we created it (standalone mode)
    // to prevent it from being deallocated?
    // In standalone, we leak it or keep it here.
    #[allow(dead_code)]
    _owned_window: Option<Retained<AnyObject>>,
    // CVDisplayLink for vsync rendering
    display_link: Option<CVDisplayLinkRef>,
}

/// CVDisplayLink callback - called at display refresh rate (~60 FPS)
extern "C" fn display_link_callback(
    _display_link: CVDisplayLinkRef,
    _in_now: *const CVTimeStamp,
    _in_output_time: *const CVTimeStamp,
    _flags_in: CVOptionFlags,
    _flags_out: *mut CVOptionFlags,
    user_info: *mut c_void,
) -> CVReturn {
    unsafe {
        crate::EventRouter::unsafe_post_from_ptr(
            user_info as *const crate::EventRouter,
            crate::UIEvent::RenderFrame,
        );
    }
    K_CV_RETURN_SUCCESS
}

impl crate::NativeWindow for MacOSWindow {
    unsafe fn attach(parent: *mut c_void) -> Result<Self> {
        // Get main thread marker (required for AppKit)
        let _mtm = MainThreadMarker::new().context("Must be called on main thread")?;

        // Cast parent to NSView
        let parent_view = parent as *mut AnyObject;

        // If parent is null, we are in Standalone mode and need to create a window
        let (view_to_attach, _owned_window) = if parent_view.is_null() {
            // Create a new NSWindow
            let window_rect = NSRect {
                origin: NSPoint { x: 100.0, y: 100.0 },
                size: NSSize {
                    width: 800.0,
                    height: 600.0,
                },
            };
            let style_mask: u64 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3); // Titled | Closable | Miniaturizable | Resizable
            let ns_window_class = AnyClass::get("NSWindow").expect("NSWindow class not found");
            let window: Retained<AnyObject> = objc2::msg_send_id![
                objc2::msg_send_id![ns_window_class, alloc],
                initWithContentRect: window_rect
                styleMask: style_mask
                backing: 2u64 // NSBackingStoreBuffered
                defer: false
            ];

            let title = objc2_foundation::NSString::from_str("splug - Standalone");
            let _: () = objc2::msg_send![&*window, setTitle: &*title];
            let _: () =
                objc2::msg_send![&*window, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];

            // Use the window's content view (it's created automatically usually, but let's check)
            let content_view: *mut AnyObject = objc2::msg_send![&*window, contentView];
            (content_view, Some(window))
        } else {
            (parent_view, None)
        };

        // Create an instance of our custom RustPluginView class
        let view_class = get_rust_view_class();

        // Allocate and init new view using new selector
        let view: Retained<AnyObject> = unsafe { objc2::msg_send_id![view_class, new] };

        // For resizing, we need to match the parent
        let parent_ref = unsafe { &*view_to_attach };
        let bounds: NSRect = objc2::msg_send![parent_ref, bounds];
        let width = bounds.size.width as u32;
        let height = bounds.size.height as u32;

        //  Set our view's frame to match parent
        let _: () = objc2::msg_send![&*view, setFrame: bounds];

        // Add as subview
        let _: () = objc2::msg_send![parent_ref, addSubview: &*view];

        // Get scale factor - try to get from window first
        let window_ptr: *mut AnyObject = objc2::msg_send![parent_ref, window];
        let scale_factor: f64 = if !window_ptr.is_null() {
            objc2::msg_send![window_ptr, backingScaleFactor]
        } else {
            // Fallback: get main screen
            let ns_screen_class = objc2::class!(NSScreen);
            let main_screen_ptr: *mut AnyObject = objc2::msg_send![ns_screen_class, mainScreen];
            if !main_screen_ptr.is_null() {
                objc2::msg_send![main_screen_ptr, backingScaleFactor]
            } else {
                1.0
            }
        };

        // Create MacOSWindow with Boxed EventRouter for stable address
        let mut window = MacOSWindow {
            view,
            width,
            height,
            scale_factor,
            event_router: Box::new(crate::EventRouter::new()),
            _owned_window,
            display_link: None, // Will be created after setting up event router
        };

        // Store EventRouter pointer in view's associated objects so event handlers can access it
        unsafe {
            #[allow(deprecated)]
            use objc2::ffi::object_setInstanceVariable;
            use std::ffi::CString;

            let key = CString::new(EVENT_ROUTER_KEY).unwrap();
            // Get pointer to the EventRouter on the HEAP, not the Box on the stack
            let router_ptr = &*window.event_router as *const crate::EventRouter as *mut c_void;

            let view_ptr = Retained::as_ptr(&window.view);
            #[allow(deprecated)]
            object_setInstanceVariable(
                view_ptr as *mut objc2::runtime::AnyObject as *mut objc2::ffi::objc_object,
                key.as_ptr(),
                router_ptr,
            );

            // Create and start CVDisplayLink for vsync rendering
            let mut display_link: CVDisplayLinkRef = std::ptr::null_mut();
            if CVDisplayLinkCreateWithActiveCGDisplays(&mut display_link) == K_CV_RETURN_SUCCESS {
                let callback_result =
                    CVDisplayLinkSetOutputCallback(display_link, display_link_callback, router_ptr);
                if callback_result == K_CV_RETURN_SUCCESS {
                    CVDisplayLinkStart(display_link);
                    window.display_link = Some(display_link);
                }
            }
        }

        Ok(window)
    }

    fn get_raw_handle(&self) -> RawWindowHandle {
        let ptr = Retained::as_ptr(&self.view) as *mut c_void;
        let view_ptr = std::ptr::NonNull::new(ptr).expect("View pointer null");

        let handle = AppKitWindowHandle::new(view_ptr);
        RawWindowHandle::AppKit(handle)
    }

    fn get_raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        let handle = raw_window_handle::AppKitDisplayHandle::new();
        raw_window_handle::RawDisplayHandle::AppKit(handle)
    }

    fn set_size(&mut self, width: u32, height: u32) -> Result<()> {
        self.width = width;
        self.height = height;

        unsafe {
            // Calculate new frame size
            // For NSWindow (standalone), we need to account for title bar potentially?
            // Usually valid to just set content size.

            if let Some(ref window) = self._owned_window {
                // Standalone: Resize the WINDOW
                let new_size = NSSize {
                    width: width as f64,
                    height: height as f64,
                };
                let _: () = objc2::msg_send![window, setContentSize: new_size];

                // The view should autoresize if configured, or we assume it matches content view.
                // But our view is a subview of content view?
                // In attach: `objc2::msg_send![parent_ref, addSubview: &*view];` where parent_ref is window.contentView.
                // We should also resize the view to match.
            }

            // Resize the view (always)
            let current_frame: NSRect = objc2::msg_send![&*self.view, frame];
            let new_frame = NSRect {
                origin: current_frame.origin,
                size: NSSize {
                    width: width as f64,
                    height: height as f64,
                },
            };
            let _: () = objc2::msg_send![&*self.view, setFrame: new_frame];
        }

        Ok(())
    }

    fn get_scale_factor(&self) -> f64 {
        self.scale_factor
    }

    fn closed(&self) -> bool {
        unsafe {
            // Check if view has a window
            let window_ptr: *mut AnyObject = objc2::msg_send![&*self.view, window];
            if window_ptr.is_null() {
                return true;
            }

            // Window exists if it's visible OR minimized
            // If it's closed, isVisible is false AND isMiniaturized is false
            let is_visible: bool = objc2::msg_send![window_ptr, isVisible];
            let is_minimized: bool = objc2::msg_send![window_ptr, isMiniaturized];

            !(is_visible || is_minimized)
        }
    }

    fn is_visible(&self) -> bool {
        // Check if view has a window AND that window is visible
        unsafe {
            let window_ptr: *mut AnyObject = objc2::msg_send![&*self.view, window];
            if window_ptr.is_null() {
                return false;
            }
            let is_visible: bool = objc2::msg_send![window_ptr, isVisible];
            is_visible
        }
    }

    fn event_router(&mut self) -> &mut crate::EventRouter {
        &mut self.event_router
    }
}

impl Drop for MacOSWindow {
    fn drop(&mut self) {
        // Stop and release CVDisplayLink
        if let Some(display_link) = self.display_link {
            unsafe {
                CVDisplayLinkStop(display_link);
                CVDisplayLinkRelease(display_link);
            }
        }

        // Remove from superview when dropped
        unsafe {
            let _: () = objc2::msg_send![&*self.view, removeFromSuperview];
        }
    }
}

impl MacOSWindow {
    /// Set event callback for handling UI events
    ///
    /// This should be called on the main thread.
    /// Events will be delivered on the main thread as well.
    pub fn set_event_callback<F>(&mut self, callback: F)
    where
        F: FnMut(crate::UIEvent) + Send + 'static,
    {
        self.event_router.set_callback(callback);

        // TODO Phase 2.3: Create NSView subclass to deliver events
        // Will need to:
        // 1. Use objc2::declare::ClassBuilder to create RustPluginView subclass
        // 2. Override acceptsFirstResponder to return YES
        // 3. Override mouseDown:, mouseUp:, mouseMoved:, etc.
        // 4. Store Rust callback pointer in associated object
        // 5. Call Rust callback from Objective-C method implementations
    }

    /// Trigger an event (for testing or manual event injection)
    #[allow(dead_code)]
    fn trigger_event(&mut self, event: crate::UIEvent) {
        self.event_router.route_event(event);
    }

    /// Helper to access the EventRouter (for standalone RPC)
    pub fn event_router_mut(&mut self) -> &mut crate::EventRouter {
        &mut self.event_router
    }
}

// ============================================================================
// MacOSApp Implementation
// ============================================================================

/// macOS application wrapper
pub struct MacOSApp {
    ns_app: Retained<AnyObject>,
}

impl crate::App for MacOSApp {
    fn init() -> Result<Self> {
        unsafe {
            let ns_app_class = AnyClass::get("NSApplication")
                .context("NSApplication class not found - is AppKit linked?")?;
            let app: Retained<AnyObject> = objc2::msg_send_id![ns_app_class, sharedApplication];

            // Set activation policy to regular app
            let policy: i64 = 0; // NSApplicationActivationPolicyRegular
            let _: bool = objc2::msg_send![&*app, setActivationPolicy: policy];

            let _: () = objc2::msg_send![&*app, activateIgnoringOtherApps: true];
            let _: () = objc2::msg_send![&*app, finishLaunching];

            Ok(Self { ns_app: app })
        }
    }

    fn poll_events(&self) {
        unsafe {
            objc2::rc::autoreleasepool(|_| {
                use objc2_foundation::ns_string;
                // app.nextEventMatchingMask:untilDate:inMode:dequeue:
                let event: Option<Retained<AnyObject>> = objc2::msg_send_id![
                    &*self.ns_app,
                    nextEventMatchingMask: u64::MAX // NSEventMaskAny
                    untilDate: std::ptr::null::<AnyObject>()
                    inMode: ns_string!("kCFRunLoopDefaultMode")
                    dequeue: true
                ];

                if let Some(event) = event {
                    let _: () = objc2::msg_send![&*self.ns_app, sendEvent: &*event];
                }
            });
        }
    }
}
