//! Widget Demo Test - Showcases all UI widgets
//!
//! This test demonstrates the widget framework with all 4 widgets.
//! Run with: cargo test --package gui --test widget_demo -- --nocapture

use gui::widgets::container::WidgetContainer;
use gui::widgets::layout::{FlexDirection, FlexLayout, GridLayout, ManualLayout};
use gui::widgets::{Button, EventResult, Knob, Slider, Textbox, Widget};
use pal::UIEvent;

#[test]
fn test_widget_demo_manual_layout() {
    println!("\n=== Widget Demo: Manual Layout ===\n");

    let mut container = WidgetContainer::new();

    // Create widgets with manual positioning
    let button = Button::new("Click Me!", 50.0, 50.0, 120.0, 40.0);
    let slider = Slider::new_horizontal(50.0, 120.0, 200.0, 30.0);
    let knob = Knob::new(100.0, 200.0, 30.0);
    let textbox = Textbox::new(50.0, 280.0, 250.0, 35.0).with_placeholder("Enter text...");

    println!("Created widgets:");
    println!("  - Button at (50, 50)");
    println!("  - Slider at (50, 120)");
    println!("  - Knob at (100, 200)");
    println!("  - Textbox at (50, 280)");

    container.add_widget(Box::new(button));
    container.add_widget(Box::new(slider));
    container.add_widget(Box::new(knob));
    container.add_widget(Box::new(textbox));

    container.set_layout(Box::new(ManualLayout::new()));

    println!("\nWidget count: {}", 4);
    println!("Layout: Manual (explicit positioning)");
}

#[test]
fn test_widget_demo_grid_layout() {
    println!("\n=== Widget Demo: Grid Layout ===\n");

    let mut container = WidgetContainer::new();

    // Create widgets for grid layout (2x2)
    let button1 = Button::new("Button 1", 0.0, 0.0, 100.0, 40.0);
    let button2 = Button::new("Button 2", 0.0, 0.0, 100.0, 40.0);
    let slider_h = Slider::new_horizontal(0.0, 0.0, 150.0, 30.0);
    let slider_v = Slider::new_vertical(0.0, 0.0, 30.0, 100.0);

    container.add_widget(Box::new(button1));
    container.add_widget(Box::new(button2));
    container.add_widget(Box::new(slider_h));
    container.add_widget(Box::new(slider_v));

    // 2 rows x 2 columns
    let layout = GridLayout::new(2, 2);
    container.set_layout(Box::new(layout));
    container.apply_layout(400.0, 300.0);

    println!("Created 2x2 grid:");
    println!("  - Row 1: Button 1, Button 2");
    println!("  - Row 2: H-Slider, V-Slider");
    println!("  Spacing: 10px (default), Padding: 10px (default)");
    println!("  Window: 400x300");
}

#[test]
fn test_widget_demo_flex_layout() {
    println!("\n=== Widget Demo: Flex Layout ===\n");

    let mut container = WidgetContainer::new();

    // Create widgets for flex layout
    let button = Button::new("Flex Button", 0.0, 0.0, 120.0, 40.0);
    let knob1 = Knob::new(0.0, 0.0, 30.0);
    let knob2 = Knob::new(0.0, 0.0, 30.0);
    let textbox = Textbox::new(0.0, 0.0, 200.0, 35.0).with_placeholder("Flex textbox");

    container.add_widget(Box::new(button));
    container.add_widget(Box::new(knob1));
    container.add_widget(Box::new(knob2));
    container.add_widget(Box::new(textbox));

    // Column layout with custom spacing and padding
    let layout = FlexLayout::new(FlexDirection::Column)
        .with_spacing(15.0)
        .with_padding(40.0);

    container.set_layout(Box::new(layout));
    container.apply_layout(400.0, 500.0);

    println!("Created flex column:");
    println!("  - Button");
    println!("  - Knob 1");
    println!("  - Knob 2");
    println!("  - Textbox");
    println!("  Direction: Column, Spacing: 15px, Padding: 40px");
}

#[test]
fn test_widget_demo_interactions() {
    println!("\n=== Widget Demo: Interactive Behavior ===\n");

    let mut container = WidgetContainer::new();

    // Create interactive widgets
    let button = Button::new("Interactive Button", 50.0, 50.0, 150.0, 40.0);
    let mut slider = Slider::new_horizontal(50.0, 120.0, 200.0, 30.0);
    slider.set_value(0.75);

    let mut knob = Knob::new(150.0, 200.0, 40.0);
    knob.set_value(0.5);

    let mut textbox = Textbox::new(50.0, 300.0, 250.0, 35.0);
    textbox.set_text("Hello, widgets!");

    let button_id = button.id();
    let slider_id = slider.id();
    let knob_id = knob.id();
    let textbox_id = textbox.id();

    container.add_widget(Box::new(button));
    container.add_widget(Box::new(slider));
    container.add_widget(Box::new(knob));
    container.add_widget(Box::new(textbox));

    println!("Widget IDs:");
    println!("  Button: {:?}", button_id);
    println!("  Slider: {:?}", slider_id);
    println!("  Knob: {:?}", knob_id);
    println!("  Textbox: {:?}", textbox_id);

    // Simulate button click
    println!("\n--- Simulating button click ---");
    let results = container.handle_ui_event(UIEvent::MouseDown {
        x: 100.0,
        y: 70.0,
        button: 0,
    });
    println!("MouseDown results: {} events", results.len());

    let results = container.handle_ui_event(UIEvent::MouseUp {
        x: 100.0,
        y: 70.0,
        button: 0,
    });
    println!("MouseUp results: {} events", results.len());
    for result in &results {
        if let EventResult::ValueChanged(value) = result {
            println!("  Button clicked! Value: {}", value);
        }
    }

    // Simulate slider drag
    println!("\n--- Simulating slider drag ---");
    container.handle_ui_event(UIEvent::MouseDown {
        x: 150.0,
        y: 135.0,
        button: 0,
    });
    let results = container.handle_ui_event(UIEvent::MouseMove { x: 200.0, y: 135.0 });
    for result in &results {
        if let EventResult::ValueChanged(value) = result {
            println!("  Slider value: {:.2}", value);
        }
    }

    // Simulate Tab navigation
    println!("\n--- Simulating Tab key (focus navigation) ---");
    container.handle_ui_event(UIEvent::KeyDown {
        keycode: 48,
        modifiers: pal::Modifiers::empty(),
    }); // Tab
    println!("  Focused widget should change (container manages focus)");

    println!("\nDemo complete!");
}

#[test]
fn test_widget_demo_all_widgets() {
    println!("\n=== Widget Demo: All Widgets Showcase ===\n");

    let mut container = WidgetContainer::new();

    // Button examples
    let button_enabled = Button::new("Enabled", 0.0, 0.0, 100.0, 35.0);
    let mut button_disabled = Button::new("Disabled", 0.0, 0.0, 100.0, 35.0);
    button_disabled.set_enabled(false);

    // Slider examples
    let slider_h = Slider::new_horizontal(0.0, 0.0, 150.0, 25.0);
    let slider_v = Slider::new_vertical(0.0, 0.0, 25.0, 100.0);

    // Knob examples
    let knob_small = Knob::new(0.0, 0.0, 25.0);
    let knob_large = Knob::new(0.0, 0.0, 40.0);

    // Textbox examples
    let textbox1 = Textbox::new(0.0, 0.0, 180.0, 32.0).with_placeholder("Type here...");
    let textbox2 = Textbox::new(0.0, 0.0, 180.0, 32.0).with_placeholder("Another field");

    println!("All widget types:");
    println!("  Buttons: 2 (enabled, disabled)");
    println!("  Sliders: 2 (horizontal, vertical)");
    println!("  Knobs: 2 (small, large)");
    println!("  Textboxes: 2 (with placeholders)");

    container.add_widget(Box::new(button_enabled));
    container.add_widget(Box::new(button_disabled));
    container.add_widget(Box::new(slider_h));
    container.add_widget(Box::new(slider_v));
    container.add_widget(Box::new(knob_small));
    container.add_widget(Box::new(knob_large));
    container.add_widget(Box::new(textbox1));
    container.add_widget(Box::new(textbox2));

    // Use grid layout for organization
    let layout = GridLayout::new(4, 2);
    container.set_layout(Box::new(layout));
    container.apply_layout(500.0, 600.0);

    println!("\nTotal widgets: 8");
    println!("Layout: 4x2 Grid");
    println!("All widgets positioned automatically!");
}
