//! Standalone host for the splug audio plugin
//!
//! Allows running the plugin without a DAW for development.

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Import dependencies at crate level to avoid lookup issues
#[cfg(target_os = "macos")]
use pal::macos::MacOSWindow;
#[cfg(target_os = "windows")]
use pal::win32::Win32Window;

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

#[derive(Parser, Debug)]
#[command(name = "standalone")]
struct Args {
    /// Run in headless mode (no GUI)
    #[arg(long)]
    headless: bool,

    /// Enable debug RPC server
    #[arg(long)]
    debug_server: bool,
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
    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    tracing::info!("splug standalone host v{}", env!("CARGO_PKG_VERSION"));

    let args = Args::parse();

    if args.headless {
        tracing::info!("Running in headless mode...");
        run_headless(&args)?;
    } else {
        tracing::info!("Running with GUI...");
        run_with_gui(&args)?;
    }

    Ok(())
}

fn run_headless(args: &Args) -> Result<()> {
    use crossbeam_channel;
    use debug_server;

    // Use a dummy channel if debug server not enabled?
    // Or just sleep loop.
    let (tx, rx) = crossbeam_channel::unbounded();

    let _lockfile_guard = if args.debug_server {
        let pid = std::process::id();
        let temp_dir = std::env::temp_dir();
        let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

        match debug_server::RpcServer::start(0, tx) {
            Ok(port) => {
                tracing::info!("RPC Server started on port {}", port);
                let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
                std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
                Some(lockfile_path)
            }
            Err(e) => {
                tracing::error!("Failed to start RPC server: {}", e);
                None
            }
        }
    } else {
        None
    };

    tracing::info!("Press Ctrl+C to exit");

    // Loop
    loop {
        // Poll RPC events from rx
        while let Ok(event) = rx.try_recv() {
            tracing::debug!("Headless received event: {:?}", event);
            if let pal::UIEvent::Quit = event {
                tracing::info!("Headless received Quit signal, exiting...");
                std::thread::sleep(std::time::Duration::from_millis(500));
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Run the plugin with GUI
/// Initializes window and rendering pipeline
fn run_with_gui(args: &Args) -> Result<()> {
    tracing::info!("Initializing macOS window...");

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

    // Flag to signal exit from callback
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    // Track window focus state
    let window_focused = Arc::new(AtomicBool::new(true)); // Start focused

    // Create a channel to buffer events so we can process them in the main loop
    // where we have mutable access to window and renderer
    let (app_tx, app_rx) = std::sync::mpsc::channel();
    let app_tx_cb = app_tx.clone();

    // Call set_callback on the EventRouter directly
    window.event_router().set_callback(move |event| {
        // Forward all events to the main loop channel
        let _ = app_tx_cb.send(event);
    });

    // Optional: Debug Server
    let _lockfile_guard = if args.debug_server {
        use crossbeam_channel;
        let (tx, rx) = crossbeam_channel::unbounded();

        // Connect RPC events to window event router
        window.event_router().set_event_receiver(rx);

        let pid = std::process::id();
        let temp_dir = std::env::temp_dir();
        let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

        match debug_server::RpcServer::start(0, tx) {
            Ok(port) => {
                tracing::info!("RPC Server started on port {}", port);
                let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
                if let Err(e) = std::fs::write(&lockfile_path, json) {
                    tracing::error!("Failed to write lockfile: {}", e);
                    None
                } else {
                    Some(lockfile_path)
                }
            }
            Err(e) => {
                tracing::error!("Failed to start RPC server: {}", e);
                None
            }
        }
    } else {
        None
    };

    tracing::info!("Window opened");

    let mut last_fps_print = std::time::Instant::now();
    let startup_time = std::time::Instant::now(); // Track startup time

    loop {
        // Poll system events via PAL (routes to callback -> app_tx)
        window.event_router().poll_events();
        app.poll_events();

        // Process buffered events
        while let Ok(event) = app_rx.try_recv() {
            match event {
                UIEvent::Quit => {
                    tracing::info!("Received Quit signal, exiting...");
                    return Ok(());
                }
                UIEvent::FocusChanged(focused) => {
                    window_focused.store(focused, Ordering::Relaxed);
                }
                UIEvent::Resize(w, h) => {
                    tracing::info!("Handling Resize event: {}x{}", w, h);
                    if let Err(e) = window.set_size(w, h) {
                        tracing::error!("Failed to resize window: {}", e);
                    }
                }
                _ => {}
            }
        }

        // Render frame
        if let Err(e) = renderer.draw_frame(None) {
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
        if startup_time.elapsed().as_millis() > 500 && window.closed() {
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

    // Explicitly remove lockfile if it exists
    if let Some(path) = _lockfile_guard {
        let _ = std::fs::remove_file(path);
    }

    Ok(())
}
