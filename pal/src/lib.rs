//! # Platform Abstraction Layer (PAL)
//!
//! Provides native windowing without using `winit` or other generic GUI frameworks.
//! Handles the "parenting" handshake required by VST3/CLAP hosts.
//!
//! ## Architecture
//!
//! The PAL solves the fundamental problem that DAWs own the event loop and expect
//! plugins to create child windows, not standalone applications. Generic GUI frameworks
//! like winit assume they own the application lifecycle, which is incompatible with
//! plugin hosting.
//!
//! For standalone usage, the PAL provides an `App` trait to abstract the system event loop.
//!
//! ## Platform Support
//!
//! - **macOS**: Uses `objc2` to create NSView subclass with CAMetalLayer backing
//! - **Windows**: Uses `windows-sys` to create child HWND with proper parenting

use anyhow::Result;
use raw_window_handle::RawWindowHandle;

mod event_router;
pub use event_router::EventRouter;

/// UI Event types for input handling
#[derive(Debug, Clone, Copy, PartialEq)] // Added Clone, Copy, PartialEq for convenience
pub enum UIEvent {
    MouseDown { x: f64, y: f64, button: u32 },
    MouseUp { x: f64, y: f64, button: u32 },
    MouseMove { x: f64, y: f64 },
    KeyDown { keycode: u32 },
    KeyUp { keycode: u32 },
    Quit,
}

/// Core trait for platform-specific window implementations
pub trait NativeWindow {
    /// Attach to a parent window provided by the host
    ///
    /// # Safety
    ///
    /// The parent pointer must be valid for the lifetime of the window:
    /// - macOS: NSView* (must remain valid)
    /// - Windows: HWND (must be a valid window handle)
    unsafe fn attach(parent: *mut std::ffi::c_void) -> Result<Self>
    where
        Self: Sized;

    /// Get the raw window handle for Vulkan surface creation
    fn get_raw_handle(&self) -> RawWindowHandle;

    /// Resize the window
    ///
    /// Note: For VST3, this should only be called from the onSize callback,
    /// not directly from user input to avoid infinite resize loops.
    fn set_size(&mut self, width: u32, height: u32) -> Result<()>;

    /// Get the DPI scale factor for proper rendering
    ///
    /// - macOS: NSScreen.backingScaleFactor (e.g., 2.0 on Retina)
    /// - Windows: GetDpiForWindow result / 96.0
    fn get_scale_factor(&self) -> f64;

    /// Check if the window exists
    ///
    /// Used to exit the application when the window is closed
    fn closed(&self) -> bool;

    /// Check if the window is currently visible
    ///
    /// Used to pause rendering when the plugin UI is hidden (e.g., tab switch in DAW)
    fn is_visible(&self) -> bool;

    /// Get mutable access to the event router
    fn event_router(&mut self) -> &mut EventRouter;
}

// Platform-specific implementations
#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod win32;

// Re-export the platform-specific implementation
#[cfg(target_os = "macos")]
pub use macos::MacOSWindow;

#[cfg(target_os = "windows")]
pub use win32::Win32Window;

#[cfg(test)]
mod tests {
    #[test]
    fn test_pal_compiles() {
        // Basic compilation test
        // Real tests will require window creation in Phase 2
        assert!(true);
    }
}

/// Core trait for application lifecycle management (Standalone mode)
pub trait App {
    /// Initialize the application (e.g., NSApp)
    fn init() -> Result<Self>
    where
        Self: Sized;

    /// Poll for pending system events (non-blocking)
    fn poll_events(&self);
}

// Re-export the platform-specific implementation
#[cfg(target_os = "macos")]
pub use macos::MacOSApp;

#[cfg(target_os = "windows")]
pub use win32::Win32App;
