use anyhow::{Context, Result};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle, XlibDisplayHandle, XlibWindowHandle,
};
use std::ffi::c_void;
use std::ptr;
use x11_dl::xlib;

pub struct LinuxWindow {
    display: *mut xlib::Display,
    window: xlib::Window,
    width: u32,
    height: u32,
    event_router: Box<crate::EventRouter>,
    xlib: xlib::Xlib,
    // If we created the window (standalone), we might want to destroy it.
    // X11 resource management is manual.
    own_window: bool,
}

impl LinuxWindow {
    fn new(
        xlib: xlib::Xlib,
        display: *mut xlib::Display,
        window: xlib::Window,
        width: u32,
        height: u32,
        own_window: bool,
    ) -> Self {
        Self {
            xlib,
            display,
            window,
            width,
            height,
            event_router: Box::new(crate::EventRouter::new()),
            own_window,
        }
    }
}

impl crate::NativeWindow for LinuxWindow {
    unsafe fn attach(parent: *mut c_void) -> Result<Self> {
        // Load Xlib dynamically
        let xlib = xlib::Xlib::open().context("Failed to load Xlib")?;

        // Open display
        let display = (xlib.XOpenDisplay)(ptr::null());
        if display.is_null() {
            anyhow::bail!("Failed to open X display");
        }

        let screen = (xlib.XDefaultScreen)(display);
        let root = (xlib.XRootWindow)(display, screen);
        let visual = (xlib.XDefaultVisual)(display, screen);
        let depth = (xlib.XDefaultDepth)(display, screen);

        let mut attributes: xlib::XSetWindowAttributes = std::mem::zeroed();
        attributes.background_pixel = (xlib.XWhitePixel)(display, screen);
        attributes.event_mask = xlib::ExposureMask
            | xlib::KeyPressMask
            | xlib::KeyReleaseMask
            | xlib::ButtonPressMask
            | xlib::ButtonReleaseMask
            | xlib::PointerMotionMask
            | xlib::StructureNotifyMask;

        let parent_window = if parent.is_null() {
            root // Standalone
        } else {
            parent as xlib::Window // VST3/CLAP parent
        };

        let width = 800;
        let height = 600;

        // Create window
        let window = (xlib.XCreateWindow)(
            display,
            parent_window,
            0,
            0,
            width,
            height,
            0,
            depth,
            xlib::InputOutput as u32,
            visual,
            xlib::CWBackPixel | xlib::CWEventMask,
            &mut attributes,
        );

        if parent.is_null() {
            // Set title for standalone (requires CString)
            let title = std::ffi::CString::new("Splug Audio Plugin").unwrap();
            (xlib.XStoreName)(display, window, title.as_ptr());

            // Map window (show it)
            (xlib.XMapWindow)(display, window);
            (xlib.XFlush)(display);
        }

        Ok(LinuxWindow::new(
            xlib,
            display,
            window,
            width,
            height,
            parent.is_null(),
        ))
    }

    fn get_raw_handle(&self) -> RawWindowHandle {
        let handle = XlibWindowHandle::new(self.window);
        RawWindowHandle::Xlib(handle)
    }

    fn get_raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        let handle = XlibDisplayHandle::new(
            Some(std::ptr::NonNull::new(self.display as *mut _).unwrap()),
            0,
        );
        raw_window_handle::RawDisplayHandle::Xlib(handle)
    }

    fn set_size(&mut self, width: u32, height: u32) -> Result<()> {
        self.width = width;
        self.height = height;
        unsafe {
            (self.xlib.XResizeWindow)(self.display, self.window, width, height);
            (self.xlib.XFlush)(self.display);
        }
        Ok(())
    }

    fn get_scale_factor(&self) -> f64 {
        // X11 usually defaults to 96 DPI (1.0).
        // For accurate DPI, one needs to query Xft.dpi or monitor properties.
        // Returning 1.0 is safe for basicheadless/xvfb.
        1.0
    }

    fn closed(&self) -> bool {
        // In X11, closing is an event (ClientMessage/DestroyNotify).
        // Since we poll events elsewhere, we would track a flag here.
        // For simplicity/headless, returning false (never closed implicitly)
        // or checking an internal flag if we implemented the event loop fully.
        // For now:
        false
    }

    fn is_visible(&self) -> bool {
        true
    }

    fn event_router(&mut self) -> &mut crate::EventRouter {
        &mut self.event_router
    }
}

// Implement HasWindowHandle and HasDisplayHandle for LinuxWindow
// to allow Vulkan context creation (which expects these traits)
impl HasWindowHandle for LinuxWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let handle = XlibWindowHandle::new(self.window);
        unsafe { Ok(WindowHandle::borrow_raw(RawWindowHandle::Xlib(handle))) }
    }
}

impl HasDisplayHandle for LinuxWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let handle = XlibDisplayHandle::new(
            Some(std::ptr::NonNull::new(self.display as *mut _).unwrap()),
            0,
        );
        unsafe { Ok(DisplayHandle::borrow_raw(RawDisplayHandle::Xlib(handle))) }
    }
}

impl Drop for LinuxWindow {
    fn drop(&mut self) {
        unsafe {
            if self.own_window {
                (self.xlib.XDestroyWindow)(self.display, self.window);
            }
            // (self.xlib.XCloseDisplay)(self.display); // Closing display might be risky if shared?
            // Usually safe if we opened it.
        }
    }
}

pub struct LinuxApp {
    _xlib: xlib::Xlib,
}

impl crate::App for LinuxApp {
    fn init() -> Result<Self> {
        let xlib = xlib::Xlib::open().context("Failed to load Xlib")?;
        Ok(Self { _xlib: xlib })
    }

    fn poll_events(&self) {
        // Polling events in X11 typically requires the Display pointer.
        // The App trait design assumes the App owns the event loop,
        // but X11 events are tied to the *Display connection*, which is held by the Window currently.
        // In a real app, App should probably create the Display and pass it to Window.
        //
        // For our Headless/Docker E2E, we might just stub this or require Window to pump events.
        //
        // NOTE: Since the current architecture separates NativeWindow and App,
        // and NativeWindow calls XOpenDisplay, each window has its own connection.
        // If we want to poll standalone events, we'd need access to that display.
        //
        // However, `poll_events` in `App` is for standalone mode.
        // In standalone `main.rs`, we create `App::init()` then `Window::attach(null)`.
        //
        // For now, let's leave this empty or stubbed as we are primarily focused on the Window
        // being usable for Vulkan rendering in tests.
    }
}
