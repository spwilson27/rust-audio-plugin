//! Standalone host for the splug audio plugin
//!
//! Allows running the plugin without a DAW for development.

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Use the library crate for audio
use standalone::audio;

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
        Ok(
            unsafe {
                raw_window_handle::DisplayHandle::borrow_raw(self.0.get_raw_display_handle())
            },
        )
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
    let log_dir = std::env::temp_dir();
    let log_file = tracing_appender::rolling::never(&log_dir, "splug.log");
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
            Ok((port, _)) => {
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
    tracing::info!("Initializing window...");

    use gui::widgets::Widget;
    use pal::{App, NativeWindow, UIEvent}; // Import traits

    // 1. Initialize Application via PAL
    // Use Box<dyn> to hold the platform-specific implementation
    let app: Box<dyn App> = Box::new(pal::AppImpl::init()?);

    // 2. Create the window via PAL
    // Safety: Passing null pointer is valid for standalone initialization
    let mut window: Box<dyn NativeWindow> =
        Box::new(unsafe { pal::Window::attach(std::ptr::null_mut())? });

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

    // 3. Initialize Audio Engine
    tracing::info!("Initializing Audio Engine...");
    let mut audio_host = audio::StandaloneAudioHost::new().context("Failed to init audio host")?;

    // Pick default output device
    let backend = audio_host.get_backend();
    let output_devices = backend.enumerate_output_devices();
    if let Some(device) = output_devices.first() {
        tracing::info!("Starting audio on device: {}", device.name);
        // Use NoOpProcessor for now (silence)
        let processor = audio_core::processor::NoOpProcessor;
        audio_host
            .start_audio(&device.name, 44100, 512, processor)
            .context("Failed to start audio")?;
    } else {
        tracing::warn!("No output devices found!");
    }

    // Optional: Debug Server
    let (_lockfile_guard, ready_notify) = if args.debug_server {
        use crossbeam_channel;
        let (tx, rx) = crossbeam_channel::unbounded();

        // Connect RPC events to window event router
        window.event_router().set_event_receiver(rx);

        let pid = std::process::id();
        let temp_dir = std::env::temp_dir();
        let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

        match debug_server::RpcServer::start(0, tx) {
            Ok((port, notify)) => {
                tracing::info!("RPC Server started on port {}", port);
                let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
                if let Err(e) = std::fs::write(&lockfile_path, json) {
                    tracing::error!("Failed to write lockfile: {}", e);
                    (None, None)
                } else {
                    (Some(lockfile_path), Some(notify))
                }
            }
            Err(e) => {
                tracing::error!("Failed to start RPC server: {}", e);
                (None, None)
            }
        }
    } else {
        (None, None)
    };

    tracing::info!("Window opened");

    // Signal app ready if debug server enabled
    if let Some(notify) = ready_notify {
        notify.notify_one();
    }

    // 4. Initialize UI
    let mut widgets = gui::widgets::container::WidgetContainer::new();
    let layout = gui::widgets::layout::FlexLayout::new(gui::widgets::layout::FlexDirection::Column)
        .with_spacing(10.0)
        .with_padding(20.0);
    widgets.set_layout(Box::new(layout));

    // Define Widgets
    // Output Device
    let output_devices = audio_host.get_backend().enumerate_output_devices();
    let device_names: Vec<String> = output_devices.iter().map(|d| d.name.clone()).collect();

    // Pick initial index
    let initial_device_idx = if let Some(first) = output_devices.first() {
        device_names
            .iter()
            .position(|n| n == &first.name)
            .unwrap_or(0)
    } else {
        0
    };

    let mut device_selector = gui::widgets::Selector::new(0.0, 0.0, 300.0, 40.0);
    device_selector.set_items(device_names.clone());
    device_selector.set_selected_index(initial_device_idx);
    let device_selector_id = Some(device_selector.id());
    widgets.add_widget(Box::new(device_selector));

    // Sample Rate
    let sample_rates = vec![
        "44100".to_string(),
        "48000".to_string(),
        "88200".to_string(),
        "96000".to_string(),
    ];
    let mut sr_selector = gui::widgets::Selector::new(0.0, 0.0, 300.0, 40.0);
    sr_selector.set_items(sample_rates.clone());
    sr_selector.set_selected_index(0); // Default 44100
    let sr_selector_id = Some(sr_selector.id());
    widgets.add_widget(Box::new(sr_selector));

    // Buffer Size
    let buffer_sizes = vec![
        "64".to_string(),
        "128".to_string(),
        "256".to_string(),
        "512".to_string(),
        "1024".to_string(),
    ];
    let mut buf_selector = gui::widgets::Selector::new(0.0, 0.0, 300.0, 40.0);
    buf_selector.set_items(buffer_sizes.clone());
    buf_selector.set_selected_index(3); // Default 512
    let buf_selector_id = Some(buf_selector.id());
    widgets.add_widget(Box::new(buf_selector));

    // Layout
    widgets.apply_layout(800.0, 600.0);

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
                    } else {
                        // Resize widget container
                        if let Err(e) = renderer.resize(w, h) {
                            tracing::error!("Failed to resize renderer: {}", e);
                        }
                        widgets.apply_layout(w as f32, h as f32);
                    }
                }
                // Forward other events to UI
                evt => {
                    let results = widgets.handle_ui_event(evt);
                    for (id, result) in results {
                        if let gui::widgets::EventResult::ValueChanged(_) = result {
                            // Check if any selector changed
                            if Some(id) == device_selector_id
                                || Some(id) == sr_selector_id
                                || Some(id) == buf_selector_id
                            {
                                // Helper to get selector string value
                                let get_val =
                                    |wid: Option<gui::widgets::WidgetId>| -> Option<String> {
                                        wid.and_then(|id| {
                                            widgets
                                                .get_widget(id)
                                                .and_then(|w| {
                                                    w.as_any()
                                                        .downcast_ref::<gui::widgets::Selector>()
                                                })
                                                .and_then(|s| s.selected_item())
                                                .map(|s| s.to_string())
                                        })
                                    };

                                if let (Some(device), Some(sr_str), Some(buf_str)) = (
                                    get_val(device_selector_id),
                                    get_val(sr_selector_id),
                                    get_val(buf_selector_id),
                                ) {
                                    if let (Ok(sr), Ok(buf)) =
                                        (sr_str.parse::<u32>(), buf_str.parse::<u32>())
                                    {
                                        tracing::info!(
                                            "Restarting audio: Device={}, SR={}, Buf={}",
                                            device,
                                            sr,
                                            buf
                                        );
                                        let processor = audio_core::processor::NoOpProcessor;
                                        if let Err(e) =
                                            audio_host.start_audio(&device, sr, buf, processor)
                                        {
                                            tracing::error!("Failed to restart audio: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render framer
        if let Err(e) = renderer.draw_frame(Some(&widgets)) {
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
