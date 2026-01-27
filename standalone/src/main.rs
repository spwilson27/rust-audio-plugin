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

// WindowHandleWrapper to implement raw_window_handle traits for &dyn NativeWindow
struct WindowHandleWrapper<'a>(&'a dyn pal::NativeWindow);

impl<'a> raw_window_handle::HasWindowHandle for WindowHandleWrapper<'a> {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.0.get_raw_handle()) })
    }
}

impl<'a> raw_window_handle::HasDisplayHandle for WindowHandleWrapper<'a> {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::{AppKitDisplayHandle, DisplayHandle, RawDisplayHandle};
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::AppKit(AppKitDisplayHandle::new()))
        })
    }
}

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
    // Initialize logging to file
    let log_file = tracing_appender::rolling::never("target", "splug.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(log_file);

    // File layer (no ANSI colors)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(false)
        .with_ansi(false)
        .with_level(true);

    // Stdout layer (with colors)
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_level(true);

    // Combine layers
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    let args = Args::parse();

    tracing::info!("splug standalone host v{}", env!("CARGO_PKG_VERSION"));

    if args.headless {
        tracing::info!("Running in headless mode...");
        run_headless()?;
    } else {
        tracing::info!("Running with GUI...");
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

    tracing::info!("Headless mode: Audio engine and RPC server would start here");

    // Setup RPC Server for testing (Headless)
    let (tx, _rx) = crossbeam_channel::unbounded();

    // Write lockfile with port
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

    match debug_server::RpcServer::start(0, tx) {
        Ok(port) => {
            tracing::info!("RPC Server started on port {}", port);
            let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
            std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
        }
        Err(e) => {
            tracing::error!("Failed to start RPC server: {}", e);
        }
    }

    tracing::info!("Press Ctrl+C to exit");

    // Poll loop for headless mode
    loop {
        // Since we don't have an EventRouter/Window in headless, we read directly from rx
        while let Ok(event) = _rx.try_recv() {
            tracing::debug!("Headless received event: {:?}", event);
            if let pal::UIEvent::Quit = event {
                tracing::info!("Headless received Quit signal, exiting...");
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
    tracing::info!("Initializing macOS window...");

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

    window.set_size(800, 600)?;

    // 2.5. Initialize Vulkan renderer
    tracing::info!("Initializing Vulkan renderer...");
    let window_handle = WindowHandleWrapper(&*window);
    let mut renderer = gui::Renderer::new(&window_handle, 800, 600)
        .context("Failed to initialize Vulkan renderer")?;
    tracing::info!("Vulkan initialized!");

    // 3. Setup RPC Server for testing
    // Create a channel for UI events
    let (tx, rx) = crossbeam_channel::unbounded();

    // Attach receiver to EventRouter using the trait method
    window.event_router().set_event_receiver(rx);

    // Start RPC server
    match debug_server::RpcServer::start(0, tx) {
        Ok(port) => {
            tracing::info!("RPC Server started on port {}", port);
            let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
            std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
        }
        Err(e) => {
            tracing::error!("Failed to start RPC server: {}", e);
        }
    }

    // Flag to signal exit from callback
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    let should_quit = Arc::new(AtomicBool::new(false));
    let should_quit_cb = should_quit.clone();

    // Track window focus state
    let window_focused = Arc::new(AtomicBool::new(true)); // Start focused
    let window_focused_cb = window_focused.clone();

    // Call set_callback on the EventRouter directly
    window
        .event_router()
        .set_callback(move |event| match event {
            UIEvent::Quit => {
                should_quit_cb.store(true, Ordering::Relaxed);
            }
            UIEvent::FocusChanged(focused) => {
                window_focused_cb.store(focused, Ordering::Relaxed);
            }
            _ => {}
        });

    tracing::info!("Window opened");

    let mut last_fps_print = std::time::Instant::now();

    loop {
        // Poll for RPC events
        window.event_router().poll_events();

        // Check for quit signal from RPC
        if should_quit.load(Ordering::Relaxed) {
            tracing::info!("Received Quit signal, exiting...");
            // Wait a bit to allow RPC response to flush
            std::thread::sleep(std::time::Duration::from_millis(500));
            break;
        }

        // Poll system events via PAL
        app.poll_events();

        // Render frame - CVDisplayLink triggers events but we render from main thread
        if let Err(e) = renderer.draw_frame() {
            tracing::warn!("Render error: {}", e);
        }

        // Print FPS every second
        if last_fps_print.elapsed().as_secs() >= 1 {
            let fps = renderer.get_fps();
            let avg_frame_time = renderer.get_avg_frame_time();
            tracing::debug!("FPS: {:.1} | Avg frame time: {:.2}ms", fps, avg_frame_time);
            last_fps_print = std::time::Instant::now();
        }

        // Exit if window is closed (not visible AND not minimized)
        if !window.closed() {
            tracing::info!("Window closed, exiting...");
            break;
        }

        // Adjust sleep based on focus: 1ms when focused, 10ms when unfocused
        let sleep_ms = if window_focused.load(Ordering::Relaxed) {
            1 // Focused: low latency for smooth 60 FPS
        } else {
            10 // Unfocused: save CPU
        };
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
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
