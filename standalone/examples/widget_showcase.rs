//! Widget showcase example
//!
//! Demonstrates all 4 widgets with interactive controls and text rendering.

use anyhow::Result;
use gui::widgets::{Button, Knob, Slider, Textbox, WidgetContainer};
use gui::{GuiContext, VulkanContext};
use pal::{App, Event, Window};
use std::sync::Arc;
use tracing::{debug, info};

struct WidgetShowcaseApp {
    window: Window,
    gui_context: GuiContext,
    running: bool,
}

impl WidgetShowcaseApp {
    fn new() -> Result<Self> {
        // Initialize platform
        let app = pal::MacOsApp::init()?;

        // Create window
        let window = Window::new("Widget Showcase", 800, 600, pal::WindowType::Normal, None)?;

        // Initialize Vulkan
        let vulkan_context = VulkanContext::new(&window)?;

        // Create GUI context
        let mut gui_context = GuiContext::new(window.get_handle(), Arc::new(vulkan_context))?;

        // Setup widgets
        Self::setup_widgets(&mut gui_context)?;

        // Show window
        window.show();

        Ok(Self {
            window,
            gui_context,
            running: true,
        })
    }

    fn setup_widgets(gui_context: &mut GuiContext) -> Result<()> {
        let mut widgets = WidgetContainer::new();

        // Title area - using buttons as labels
        let title = Button::new("Widget Framework Showcase", 250.0, 20.0, 300.0, 40.0);
        widgets.add_widget(Box::new(title));

        // Row 1: Buttons
        let mut button1 = Button::new("Click Me!", 50.0, 100.0, 150.0, 40.0);
        widgets.add_widget(Box::new(button1));

        let mut button2 = Button::new("Button 2", 250.0, 100.0, 150.0, 40.0);
        button2.set_enabled(true);
        widgets.add_widget(Box::new(button2));

        let mut button3 = Button::new("Disabled", 450.0, 100.0, 150.0, 40.0);
        button3.set_enabled(false);
        widgets.add_widget(Box::new(button3));

        // Row 2: Sliders
        let label_slider = Button::new("Sliders:", 50.0, 170.0, 100.0, 30.0);
        widgets.add_widget(Box::new(label_slider));

        let mut slider_h = Slider::new_horizontal(50.0, 220.0, 300.0, 30.0);
        slider_h.set_value(0.7);
        widgets.add_widget(Box::new(slider_h));

        let mut slider_v = Slider::new_vertical(400.0, 170.0, 30.0, 150.0);
        slider_v.set_value(0.5);
        widgets.add_widget(Box::new(slider_v));

        // Row 3: Knobs
        let label_knob = Button::new("Knobs:", 50.0, 350.0, 100.0, 30.0);
        widgets.add_widget(Box::new(label_knob));

        let mut knob1 = Knob::new(125.0, 420.0, 40.0);
        knob1.set_value(0.3);
        widgets.add_widget(Box::new(knob1));

        let mut knob2 = Knob::new(250.0, 420.0, 40.0);
        knob2.set_value(0.6);
        widgets.add_widget(Box::new(knob2));

        let mut knob3 = Knob::new(375.0, 420.0, 40.0);
        knob3.set_value(0.9);
        widgets.add_widget(Box::new(knob3));

        // Row 4: Textbox
        let label_text = Button::new("Text Input:", 50.0, 500.0, 120.0, 30.0);
        widgets.add_widget(Box::new(label_text));

        let textbox = Textbox::new(190.0, 500.0, 400.0, 40.0).with_placeholder("Type here...");
        widgets.add_widget(Box::new(textbox));

        // Set the widgets
        gui_context.set_widgets(widgets);

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        info!("Widget Showcase starting...");

        while self.running {
            // Poll platform events
            match self.window.poll_event() {
                Some(Event::Close) => {
                    info!("Window closed");
                    self.running = false;
                }
                Some(event) => {
                    // Convert to UIEvent and handle
                    if let Some(ui_event) = Self::convert_event(event) {
                        let _ = self.gui_context.handle_event(ui_event);
                    }
                }
                None => {}
            }

            // Render frame
            self.gui_context.render()?;

            // Small sleep to avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
        }

        info!("Widget Showcase exiting");
        Ok(())
    }

    fn convert_event(event: Event) -> Option<gui::UIEvent> {
        use gui::{MouseButton, UIEvent};

        match event {
            Event::MouseDown { x, y, button } => Some(UIEvent::MouseDown {
                x,
                y,
                button: match button {
                    0 => MouseButton::Left,
                    1 => MouseButton::Right,
                    _ => MouseButton::Middle,
                },
            }),
            Event::MouseUp { x, y, button } => Some(UIEvent::MouseUp {
                x,
                y,
                button: match button {
                    0 => MouseButton::Left,
                    1 => MouseButton::Right,
                    _ => MouseButton::Middle,
                },
            }),
            Event::MouseMove { x, y } => Some(UIEvent::MouseMove { x, y }),
            Event::KeyDown { keycode } => Some(UIEvent::KeyDown {
                key: gui::Key::Unknown, // TODO: Map keycodes
            }),
            Event::KeyUp { keycode } => Some(UIEvent::KeyUp {
                key: gui::Key::Unknown,
            }),
            _ => None,
        }
    }
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut app = WidgetShowcaseApp::new()?;
    app.run()
}
