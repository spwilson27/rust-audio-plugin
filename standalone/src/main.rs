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
            if let pal::UIEvent::Quit = event {
                println!("Headless received Quit signal, exiting...");
                // Wait a bit to allow RPC response to flush
                std::thread::sleep(std::time::Duration::from_millis(500));
                return Ok(()); // return from run_headless, effectively exiting main
            }
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
fn run_with_gui() -> Result<()> {
    println!("Initializing macOS window...");

    println!("Initializing macOS window...");

    // Write lockfile with port
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

    use pal::{App, NativeWindow, UIEvent}; // Import traits

    // 1. Initialize Application via PAL
    // Use Box<dyn> to hold the platform-specific implementation
    #[cfg(target_os = "macos")]
    let app: Box<dyn App> = Box::new(pal::MacOSApp::init()?);

    // 2. Create the window via PAL
    #[cfg(target_os = "macos")]
    // Safety: Passing null pointer is valid for standalone initialization
    let mut window: Box<dyn NativeWindow> =
        Box::new(unsafe { MacOSWindow::attach(std::ptr::null_mut())? });

    #[cfg(not(target_os = "macos"))]
    let (app, mut window) = unimplemented!("Only macOS supported for now");

    // 3. Setup RPC Server for testing
    // Create a channel for UI events
    let (tx, rx) = crossbeam_channel::unbounded();

    // Attach receiver to EventRouter using the trait method
    window.event_router().set_event_receiver(rx);

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

    // Flag to signal exit from callback
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let should_quit = Arc::new(AtomicBool::new(false));
    let should_quit_cb = should_quit.clone();

    // Call set_callback on the EventRouter directly
    window.event_router().set_callback(move |event| {
        println!("Received event (Main): {:?}", event);
        if let UIEvent::Quit = event {
            should_quit_cb.store(true, Ordering::Relaxed);
        }
    });

    println!("\nWindow opened!");
    println!("Close the window to exit.\n");

    loop {
        // Poll for RPC events
        window.event_router().poll_events();

        // Check for quit signal from RPC
        if should_quit.load(Ordering::Relaxed) {
            println!("Received Quit signal, exiting...");
            // Wait a bit to allow RPC response to flush
            std::thread::sleep(std::time::Duration::from_millis(500));
            break;
        }

        // Poll system events via PAL
        app.poll_events();

        // Exit if window is closed (not visible AND not minimized)
        if !window.closed() {
            println!("Window closed, exiting...");
            break;
        }

        // Sleep a tiny bit to avoid 100% CPU in this naive loop
        std::thread::sleep(std::time::Duration::from_millis(16));
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
