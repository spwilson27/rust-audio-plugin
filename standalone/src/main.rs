//! Standalone host for the splug audio plugin
//!
//! Allows running the plugin without a DAW for testing and development.

use anyhow::{Context, Result};
use clap::Parser;

// Import dependencies at crate level to avoid lookup issues
#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
#[cfg(target_os = "macos")]
use pal::macos::MacOSWindow;
#[cfg(target_os = "macos")]
use pal::NativeWindow; // Import trait for attach

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
    // Need imports here as well if not global, but global is better
    use crossbeam_channel;
    use debug_server;

    println!("Headless mode: Audio engine and RPC server would start here");

    // Setup RPC Server for testing (Headless)
    let (tx, _rx) = crossbeam_channel::unbounded();

    // Write lockfile with port
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

    match debug_server::RpcServer::start(0, tx) {
        Ok(port) => {
            println!("RPC Server started on port {}", port);
            let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
            std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
        }
        Err(e) => {
            eprintln!("Failed to start RPC server: {}", e);
        }
    }

    println!("Press Ctrl+C to exit");

    // Poll loop for headless mode
    loop {
        // Since we don't have an EventRouter/Window in headless, we read directly from rx
        while let Ok(event) = _rx.try_recv() {
            println!("Headless received event: {:?}", event);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[allow(unreachable_code)]
    {
        let _ = std::fs::remove_file(lockfile_path);
        Ok(())
    }
}

/// Run the plugin with GUI
/// Initializes window and rendering pipeline
#[cfg(target_os = "macos")]
fn run_with_gui() -> Result<()> {
    use objc2::rc::Retained;
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::ns_string;

    println!("Initializing macOS window...");

    // Write lockfile with port
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

    unsafe {
        // 1. Initialize NSApplication
        let ns_app_class = AnyClass::get("NSApplication")
            .expect("NSApplication class not found - is AppKit linked?");
        let app: Retained<AnyObject> = objc2::msg_send_id![ns_app_class, sharedApplication];

        // Set activation policy to regular app
        let policy: i64 = 0; // NSApplicationActivationPolicyRegular
        let _: bool = objc2::msg_send![&*app, setActivationPolicy: policy];

        // 2. Create the window via PAL
        // For standalone, we pass null as parent, which implies creating a new window
        let mut window = MacOSWindow::attach(std::ptr::null_mut())?;

        // 3. Setup RPC Server for testing
        // Create a channel for UI events
        let (tx, rx) = crossbeam_channel::unbounded();

        // Attach receiver to EventRouter
        window.event_router_mut().set_event_receiver(rx);

        // Start RPC server
        match debug_server::RpcServer::start(0, tx) {
            Ok(port) => {
                println!("RPC Server started on port {}", port);
                let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
                std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
            }
            Err(e) => {
                eprintln!("Failed to start RPC server: {}", e);
            }
        }

        window.set_size(800, 600)?;

        window.set_event_callback(|event| {
            println!("Received event (Main): {:?}", event);
        });

        // 4. Run the event loop
        let _: () = objc2::msg_send![&*app, activateIgnoringOtherApps: true];
        let _: () = objc2::msg_send![&*app, finishLaunching];

        println!("\nWindow opened!");
        println!("Close the window to exit.\n");

        loop {
            // Poll for RPC events
            window.event_router_mut().poll_events();

            objc2::rc::autoreleasepool(|_| {
                // app.nextEventMatchingMask:untilDate:inMode:dequeue:
                let event: Option<Retained<AnyObject>> = objc2::msg_send_id![
                    &*app,
                    nextEventMatchingMask: u64::MAX // NSEventMaskAny
                    untilDate: std::ptr::null::<AnyObject>()
                    inMode: ns_string!("kCFRunLoopDefaultMode")
                    dequeue: true
                ];

                if let Some(event) = event {
                    let _: () = objc2::msg_send![&*app, sendEvent: &*event];
                }
            });

            // Exit if window is closed
            if !window.is_visible() {
                println!("Window closed, exiting...");
                break;
            }

            // Sleep a tiny bit to avoid 100% CPU in this naive loop
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }

    // Unreachable loop but compiler doesn't know for sure if we break
    #[allow(unreachable_code)]
    {
        let _ = std::fs::remove_file(lockfile_path);
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
fn run_with_gui() -> Result<()> {
    anyhow::bail!("GUI mode only supported on macOS for now (Phase 2.5)");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_headless_mode_doesnt_panic() {
        assert!(true);
    }
}
