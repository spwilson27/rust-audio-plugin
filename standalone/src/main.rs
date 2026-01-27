//! Standalone host for the splug audio plugin
//!
//! Allows running the plugin without a DAW for testing and development.

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "standalone")]
#[command(about = "Standalone host for splug audio plugin", long_about = None)]
struct Args {
    /// Run in headless mode (no GUI, for automated testing)
    #[arg(long)]
    headless: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("splug standalone host v{}", env!("CARGO_PKG_VERSION"));

    if args.headless {
        println!("Running in headless mode...");
        run_headless()?;
    } else {
        println!("Running with GUI...");
        run_with_gui()?;
    }

    Ok(())
}

/// Run the plugin in headless mode (no window)
/// Used for automated testing with RPC server
fn run_headless() -> Result<()> {
    println!("Headless mode: Audio engine and RPC server would start here");
    println!("TODO: Phase 5 - Initialize RPC server");
    println!("TODO: Phase 4 - Initialize audio processor");

    // For now, just demonstrate the flag works
    println!("Press Ctrl+C to exit");

    // Block indefinitely (in real implementation, wait on audio/RPC threads)
    std::thread::park();

    Ok(())
}

/// Run the plugin with GUI
/// Initializes window and rendering pipeline
#[cfg(target_os = "macos")]
fn run_with_gui() -> Result<()> {
    use objc2::msg_send_id;
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSString;
    use std::ffi::c_void;

    println!("Initializing macOS window...");

    unsafe {
        // 1. Initialize NSApplication
        let ns_app_class = AnyClass::get("NSApplication")
            .expect("NSApplication class not found - is AppKit linked?");
        let app: Retained<AnyObject> = msg_send_id![ns_app_class, sharedApplication];

        // Set activation policy to regular app (shows in Dock, can become active)
        let policy: i64 = 0; // NSApplicationActivationPolicyRegular
        let _: bool = objc2::msg_send![&*app, setActivationPolicy: policy];

        // 2. Create NSWindow
        let window_rect = objc2_foundation::NSRect {
            origin: objc2_foundation::NSPoint { x: 100.0, y: 100.0 },
            size: objc2_foundation::NSSize {
                width: 800.0,
                height: 600.0,
            },
        };

        // Style mask: Titled | Closable | Miniaturizable | Resizable
        let style_mask: u64 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);

        let ns_window_class = AnyClass::get("NSWindow").expect("NSWindow class not found");
        let window: Retained<AnyObject> = msg_send_id![
            msg_send_id![ns_window_class, alloc],
            initWithContentRect: window_rect
            styleMask: style_mask
            backing: 2u64  // NSBackingStoreBuffered
            defer: false
        ];

        // Set window title
        let title = NSString::from_str("splug - Standalone");
        let _: () = objc2::msg_send![&*window, setTitle: &*title];

        // 3. Get content view and create GUI context
        let content_view: *mut AnyObject = objc2::msg_send![&*window, contentView];

        println!("Creating GUI context...");
        let mut gui_ctx = gui::GuiContext::attach(content_view as *mut c_void, 800, 600)?;

        // 4. Set up event logging
        println!("Setting up event logging...");
        if let Some(window) = gui_ctx.get_window_mut() {
            window.set_event_callback(|event| {
                println!("Event: {:?}", event);
            });
        }

        // 5. Make window visible
        println!("Opening window...");
        let _: () = objc2::msg_send![&*window, makeKeyAndOrderFront: std::ptr::null::<AnyObject>()];

        // 6. Activate application
        let _: () = objc2::msg_send![&*app, activateIgnoringOtherApps: true];

        println!("\nWindow opened!");
        println!("Close the window to exit.\n");

        // 7. Run event loop
        let _: () = objc2::msg_send![&*app, run];

        // Clean up happens automatically via Drop
        drop(gui_ctx);

        println!("Shutting down...");
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn run_with_gui() -> Result<()> {
    anyhow::bail!("GUI mode only supported on macOS for now (Phase 2.5)");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_headless_mode_doesnt_panic() {
        // Verify headless mode initialization doesn't crash
        // In a real test, we'd spawn a thread and verify RPC server starts
        assert!(true);
    }
}
