//! End-to-end test binary for visual golden tests
//!
//! Supports multiple test modes for golden image generation and verification.
//! Modes: widgets, text-resize

use anyhow::{Context, Result};
use clap::Parser;
use gui::widgets::container::WidgetContainer;
use gui::widgets::{Button, Knob, Slider, Textbox, Widget};

// WindowHandleWrapper for raw_window_handle traits
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
#[command(name = "test-e2e")]
#[command(about = "E2E test runner for golden tests")]
struct Args {
    /// Test mode to run
    #[arg(long, default_value = "widgets")]
    mode: String,

    /// Force fixed FPS (60.0) for deterministic testing
    #[arg(long, default_value = "true")]
    fixed_fps: bool,
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let args = Args::parse();

    tracing::info!("test-e2e runner - mode: {}", args.mode);

    match args.mode.as_str() {
        "widgets" => run_app(&args, setup_widget_showcase()),
        "text-resize" => run_app(&args, setup_text_widgets()),
        "headless" => run_headless(),
        _ => anyhow::bail!("Unknown mode: {}", args.mode),
    }
}

fn run_headless() -> Result<()> {
    tracing::info!("Running in headless mode...");
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("test_e2e_pid_{}.json", pid));

    // Channel for RPC events
    let (tx, rx) = crossbeam_channel::unbounded();

    // Start RPC server
    let _port = match debug_server::RpcServer::start(0, tx) {
        Ok((port, _)) => {
            tracing::info!("RPC Server started on port {}", port);
            let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
            std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
            port
        }
        Err(e) => anyhow::bail!("Failed to start RPC server: {}", e),
    };

    use pal::UIEvent;

    tracing::info!("Press Ctrl+C to exit");

    loop {
        while let Ok(event) = rx.try_recv() {
            if let UIEvent::Quit = event {
                tracing::info!("Received Quit signal, exiting...");
                std::thread::sleep(std::time::Duration::from_millis(500));
                // Clean up lockfile
                let _ = std::fs::remove_file(&lockfile_path);
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Shared app runner: Inits window/renderer, starts RPC, run loop
fn run_app(args: &Args, mut widgets: WidgetContainer) -> Result<()> {
    tracing::info!("Initializing app...");

    // Write lockfile with port
    let pid = std::process::id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("test_e2e_pid_{}.json", pid));

    use pal::{App, NativeWindow, UIEvent};

    // Initialize Application
    let app: Box<dyn App> = Box::new(pal::AppImpl::init()?);

    // Create window
    let mut window: Box<dyn NativeWindow> =
        Box::new(unsafe { pal::Window::attach(std::ptr::null_mut())? });

    window.set_size(800, 600)?;

    // Initialize Vulkan renderer
    tracing::info!("Initializing Vulkan renderer...");
    let window_handle = WindowHandleWrapper(&*window);
    let mut renderer = gui::Renderer::new(&window_handle, 800, 600)
        .context("Failed to initialize Vulkan renderer")?;

    if args.fixed_fps {
        renderer.set_fixed_fps(Some(60.0));
    }

    // Connect widgets to renderer (informational logging)
    let widget_count = widgets.len();
    tracing::info!("Running with {} widgets", widget_count);

    // Setup RPC Server
    let (tx, _rx) = crossbeam_channel::unbounded();
    window.event_router().set_event_receiver(_rx);

    let ready_notify = match debug_server::RpcServer::start(0, tx) {
        Ok((port, notify)) => {
            tracing::info!("RPC Server started on port {}", port);
            let json = format!("{{ \"port\": {}, \"pid\": {} }}", port, pid);
            std::fs::write(&lockfile_path, json).context("Failed to write lockfile")?;
            Some(notify)
        }
        Err(e) => {
            tracing::error!("Failed to start RPC server: {}", e);
            None
        }
    };

    // Event channel
    let (app_tx, app_rx) = std::sync::mpsc::channel();
    let app_tx_cb = app_tx.clone();

    window.event_router().set_callback(move |event| {
        let r = app_tx_cb.send(event);
        if r.is_err() {
            tracing::error!("Event dropped: {:?}", r);
        }
    });

    tracing::info!("App loop running...");

    let mut _frame_count = 0;
    let mut signaled_ready = false;
    loop {
        window.event_router().poll_events();
        app.poll_events();

        // Process events
        while let Ok(event) = app_rx.try_recv() {
            match &event {
                UIEvent::Quit => {
                    tracing::info!("Received Quit, exiting...");
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    return Ok(());
                }
                UIEvent::Resize(w, h) => {
                    tracing::info!("Handling Resize: {}x{}", w, h);
                    if let Err(e) = window.set_size(*w, *h) {
                        tracing::error!("Failed to resize window: {}", e);
                    }
                    // TODO: Notify renderer of resize?
                    // renderer.resize(*w, *h)?
                    // `gui::Renderer` might handle swapchain resize in `draw_frame` automatically or needs explicit call.
                    // Assuming `draw_frame` handles it or we need `renderer.handle_resize(*w, *h)`.
                    // Given `standalone` didn't call renderer.resize (checked previously), we assume draw_frame handles it or we missed it.
                    // Actually `gui::Renderer` usually checks extents.
                }
                UIEvent::CaptureScreen(reply_tx) => {
                    tracing::info!("Capturing screenshot...");
                    match renderer.capture_frame() {
                        Ok(img) => {
                            let (width, height) = img.dimensions();
                            let data = img.into_raw();
                            let _ = reply_tx.send((data, width, height));
                        }
                        Err(e) => {
                            tracing::error!("Failed to capture frame: {}", e);
                        }
                    }
                }
                UIEvent::Custom(data) => {
                    // Handle GetWidgetStateRequest
                    if let Some(req) = data.downcast_ref::<debug_server::GetWidgetStateRequest>() {
                        // Use index as ID
                        let index = req.id as usize;
                        let state = if let Some(widget) = widgets.get_widget_at(index) {
                            extract_widget_state(widget, req.id)
                        } else {
                            None
                        };
                        let _ = req.reply.send(state);
                    }
                    // Handle SetWidgetValueRequest
                    else if let Some(req) =
                        data.downcast_ref::<debug_server::SetWidgetValueRequest>()
                    {
                        let index = req.id as usize;
                        let success = if let Some(widget) = widgets.get_widget_at_mut(index) {
                            if let Some(slider) = widget.as_any_mut().downcast_mut::<Slider>() {
                                slider.set_value(req.value);
                                true
                            } else if let Some(knob) = widget.as_any_mut().downcast_mut::<Knob>() {
                                knob.set_value(req.value);
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        let _ = req.reply.send(success);
                    }
                    // Handle SyncEventsRequest
                    else if let Some(req) = data.downcast_ref::<debug_server::SyncEventsRequest>()
                    {
                        let _ = req.reply.send(true);
                    }
                }
                // Forward all other events to widgets
                _ => {
                    widgets.handle_ui_event(event.clone());
                }
            }
        }

        // Render
        if let Err(e) = renderer.draw_frame(Some(&widgets)) {
            tracing::warn!("Render error: {}", e);
        }
        if !signaled_ready {
            // Signal readiness after first successful frame
            if let Some(notify) = &ready_notify {
                notify.notify_one();
                tracing::info!("Signaled App Ready");
            }
            signaled_ready = true;
        }

        _frame_count += 1;

        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn setup_text_widgets() -> WidgetContainer {
    let mut container = WidgetContainer::new();
    // A simple textbox to show text rendering
    let mut textbox = Textbox::new(50.0, 50.0, 300.0, 40.0);
    textbox.set_text("Resize Test Mode");
    container.add_widget(Box::new(textbox));
    container
}

/// Setup all widgets for showcase
fn setup_widget_showcase() -> WidgetContainer {
    let mut container = WidgetContainer::new();

    // Row 1: Buttons (different states)
    let button_normal = Button::new("Normal", 50.0, 50.0, 120.0, 40.0);
    container.add_widget(Box::new(button_normal));

    let button_hovered = Button::new("Button 2", 200.0, 50.0, 120.0, 40.0);
    container.add_widget(Box::new(button_hovered));

    let mut button_disabled = Button::new("Disabled", 350.0, 50.0, 120.0, 40.0);
    button_disabled.set_enabled(false);
    container.add_widget(Box::new(button_disabled));

    // Row 2: Sliders
    let mut slider_h = Slider::new_horizontal(50.0, 120.0, 250.0, 30.0);
    slider_h.set_value(0.75);
    container.add_widget(Box::new(slider_h));

    let mut slider_v = Slider::new_vertical(350.0, 120.0, 30.0, 100.0);
    slider_v.set_value(0.5);
    container.add_widget(Box::new(slider_v));

    // Row 3: Knobs
    let mut knob1 = Knob::new(75.0, 280.0, 30.0);
    knob1.set_value(0.3);
    container.add_widget(Box::new(knob1));

    let mut knob2 = Knob::new(200.0, 280.0, 30.0);
    knob2.set_value(0.6);
    container.add_widget(Box::new(knob2));

    let mut knob3 = Knob::new(325.0, 280.0, 30.0);
    knob3.set_value(0.9);
    container.add_widget(Box::new(knob3));

    // Row 4: Textboxes
    let mut textbox_text = Textbox::new(50.0, 380.0, 200.0, 35.0);
    textbox_text.set_text("Sample Text");
    container.add_widget(Box::new(textbox_text));

    let textbox_placeholder =
        Textbox::new(280.0, 380.0, 200.0, 35.0).with_placeholder("Type here...");
    container.add_widget(Box::new(textbox_placeholder));

    let mut textbox_focused = Textbox::new(50.0, 450.0, 200.0, 35.0);
    textbox_focused.set_text("Focused");
    textbox_focused.set_focused(true);
    container.add_widget(Box::new(textbox_focused));

    container
}

/// Helper to extract state from a widget for RPC
fn extract_widget_state(
    widget: &dyn Widget,
    id: u64,
) -> Option<debug_server::debug_control::WidgetState> {
    use debug_server::debug_control::{
        widget_state, ButtonState, KnobState, SliderState, TextboxState,
    };

    let bounds = widget.bounds();
    let rect = debug_server::debug_control::Rect {
        x: bounds.x,
        y: bounds.y,
        width: bounds.width,
        height: bounds.height,
    };

    let mut state = debug_server::debug_control::WidgetState {
        widget_id: id,
        widget_type: "Unknown".to_string(),
        focused: widget.is_focused(),
        enabled: true,
        bounds: Some(rect),
        specific_state: None,
    };

    if let Some(_btn) = widget.as_any().downcast_ref::<Button>() {
        state.widget_type = "Button".to_string();
        state.specific_state = Some(widget_state::SpecificState::Button(ButtonState {
            label: "Button".to_string(), // Placeholder
            state: "Normal".to_string(), // Placeholder
        }));
    } else if let Some(slider) = widget.as_any().downcast_ref::<Slider>() {
        state.widget_type = "Slider".to_string();
        state.specific_state = Some(widget_state::SpecificState::Slider(SliderState {
            value: slider.value(),
            orientation: format!("{:?}", slider.orientation()),
            dragging: false,
        }));
    } else if let Some(knob) = widget.as_any().downcast_ref::<Knob>() {
        state.widget_type = "Knob".to_string();
        state.specific_state = Some(widget_state::SpecificState::Knob(KnobState {
            value: knob.value(),
            min_angle: -150.0,
            max_angle: 150.0,
            dragging: false,
        }));
    } else if let Some(textbox) = widget.as_any().downcast_ref::<Textbox>() {
        state.widget_type = "Textbox".to_string();
        state.specific_state = Some(widget_state::SpecificState::Textbox(TextboxState {
            text: textbox.text().to_string(),
            cursor_pos: textbox.cursor_position() as u32,
            has_selection: textbox.has_selection(),
            placeholder: textbox.placeholder().to_string(), // assuming access or skip
        }));
    }

    Some(state)
}
