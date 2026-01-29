//! Widget showcase example
//!
//! Demonstrates all 4 widgets with interactive controls and text rendering.

use anyhow::{Context, Result};
use gui::widgets::container::WidgetContainer;
use gui::widgets::{Button, Knob, Slider, Textbox};
use pal::{App, AppImpl, NativeWindow, UIEvent as PalEvent, Window};
use tracing::info;

struct WindowHandleWrapper<'a>(&'a dyn pal::NativeWindow);

impl<'a> raw_window_handle::HasWindowHandle for WindowHandleWrapper<'a> {
    fn window_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError>
    {
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(self.0.get_raw_handle()) })
    }
}

impl<'a> raw_window_handle::HasDisplayHandle for WindowHandleWrapper<'a> {
    fn display_handle(
        &self,
    ) -> std::result::Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError>
    {
        use raw_window_handle::{AppKitDisplayHandle, DisplayHandle, RawDisplayHandle};
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::AppKit(AppKitDisplayHandle::new()))
        })
    }
}

struct WidgetShowcaseApp {
    app: AppImpl,
    window: Window,
    widgets: WidgetContainer,
    renderer: gui::Renderer,
    running: bool,
}

impl WidgetShowcaseApp {
    fn new() -> Result<Self> {
        // Initialize platform
        let app = AppImpl::init()?;

        // Create window (standalone mode)
        let mut window = unsafe { Window::attach(std::ptr::null_mut())? };
        window.set_size(800, 600)?;

        // Initialize Vulkan renderer
        let wrapper = WindowHandleWrapper(&window);
        let renderer = gui::Renderer::new(&wrapper, 800, 600)
            .context("Failed to initialize Vulkan renderer")?;

        // Setup widgets
        let widgets = Self::setup_widgets();

        Ok(Self {
            app,
            window,
            widgets,
            renderer,
            running: true,
        })
    }

    fn setup_widgets() -> WidgetContainer {
        let mut widgets = WidgetContainer::new();

        // Row 1: Buttons
        let button1 = Button::new("Click Me!", 50.0, 50.0, 150.0, 40.0);
        widgets.add_widget(Box::new(button1));

        let mut button2 = Button::new("Focusable", 250.0, 50.0, 150.0, 40.0);
        button2.set_enabled(true);
        widgets.add_widget(Box::new(button2));

        let mut button3 = Button::new("Disabled", 450.0, 50.0, 150.0, 40.0);
        button3.set_enabled(false);
        widgets.add_widget(Box::new(button3));

        // Row 2: Sliders
        let mut slider_h = Slider::new_horizontal(50.0, 120.0, 300.0, 30.0);
        slider_h.set_value(0.7);
        widgets.add_widget(Box::new(slider_h));

        let mut slider_v = Slider::new_vertical(400.0, 120.0, 30.0, 150.0);
        slider_v.set_value(0.5);
        widgets.add_widget(Box::new(slider_v));

        // Row 3: Knobs
        let mut knob1 = Knob::new(125.0, 300.0, 40.0);
        knob1.set_value(0.3);
        widgets.add_widget(Box::new(knob1));

        let mut knob2 = Knob::new(250.0, 300.0, 40.0);
        knob2.set_value(0.6);
        widgets.add_widget(Box::new(knob2));

        let mut knob3 = Knob::new(375.0, 300.0, 40.0);
        knob3.set_value(0.9);
        widgets.add_widget(Box::new(knob3));

        // Row 4: Textbox
        let textbox = Textbox::new(50.0, 450.0, 400.0, 40.0).with_placeholder("Type here...");
        widgets.add_widget(Box::new(textbox));

        widgets
    }

    fn run(&mut self) -> Result<()> {
        info!("Widget Showcase starting...");

        // Setup event callback
        let (tx, rx) = std::sync::mpsc::channel();
        let tx_cb = tx.clone();
        self.window.event_router().set_callback(move |event| {
            let _ = tx_cb.send(event);
        });

        while self.running {
            // Poll events
            self.window.event_router().poll_events();
            self.app.poll_events();

            // Process events
            while let Ok(event) = rx.try_recv() {
                match event {
                    PalEvent::Quit => {
                        info!("Quit received");
                        self.running = false;
                    }
                    _ => {
                        // Forward to widgets
                        self.widgets.handle_ui_event(event);
                    }
                }
            }

            // Render frame
            self.renderer.draw_frame(Some(&self.widgets))?;

            // Small sleep
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        info!("Widget Showcase exiting");
        Ok(())
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut app = WidgetShowcaseApp::new()?;
    app.run()
}
