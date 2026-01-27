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

use anyhow::{bail, Context, Result};
use raw_window_handle::{AppKitWindowHandle, RawWindowHandle};
use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::encode::{Encode, Encoding};
use objc2::msg_send_id;
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

/// macOS window implementation using NSView
pub struct MacOSWindow {
    view: Retained<AnyObject>,
    width: u32,
    height: u32,
    scale_factor: f64,
    // Event callback - will be called when events occur
    // Using Option<Box<>> for now, Phase 2.3 will use lock-free queue
    event_callback: Option<Box<dyn FnMut(UIEvent) + Send>>,
}

// Custom UIEvent type for PAL layer (re-export from gui in Phase 2.3)
#[derive(Debug, Clone, Copy)]
pub enum UIEvent {
    MouseDown { x: f64, y: f64, button: u32 },
    MouseUp { x: f64, y: f64, button: u32 },
    MouseMove { x: f64, y: f64 },
    KeyDown { keycode: u16 },
    KeyUp { keycode: u16 },
}

impl crate::NativeWindow for MacOSWindow {
    unsafe fn attach(parent: *mut c_void) -> Result<Self> {
        // Get main thread marker (required for AppKit)
        let _mtm = MainThreadMarker::new().context("Must be called on main thread")?;

        // Cast parent to NSView
        let parent_view = parent as *mut AnyObject;
        if parent_view.is_null() {
            bail!("Parent view pointer is null");
        }

        // For now, create a basic NSView using objc2
        // We'll create a subclass in Phase 2.2 when we add event handling
        let ns_view_class = objc2::class!(NSView);

        // Allocate and init new NSView
        let view: Retained<AnyObject> = msg_send_id![ns_view_class, new];

        let parent_ref = &*parent_view;
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

        Ok(MacOSWindow {
            view,
            width,
            height,
            scale_factor,
            event_callback: None,
        })
    }

    fn get_raw_handle(&self) -> RawWindowHandle {
        let view_ptr = Retained::as_ptr(&self.view) as *mut c_void;
        let ns_view = NonNull::new(view_ptr).expect("NSView pointer should never be null");

        let handle = AppKitWindowHandle::new(ns_view);
        RawWindowHandle::AppKit(handle)
    }

    fn set_size(&mut self, width: u32, height: u32) -> Result<()> {
        self.width = width;
        self.height = height;

        // Update NSView frame using objc2 messaging
        unsafe {
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

    fn is_visible(&self) -> bool {
        // Check if view has a window (indicates it's in the view hierarchy)
        unsafe {
            let window_ptr: *mut AnyObject = objc2::msg_send![&*self.view, window];
            !window_ptr.is_null()
        }
    }
}

impl Drop for MacOSWindow {
    fn drop(&mut self) {
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
        F: FnMut(UIEvent) + Send + 'static,
    {
        self.event_callback = Some(Box::new(callback));

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
    fn trigger_event(&mut self, event: UIEvent) {
        if let Some(ref mut callback) = self.event_callback {
            callback(event);
        }
    }
}
