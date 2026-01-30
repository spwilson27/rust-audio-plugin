//! Interactive E2E test for widgets
//!
//! Verifies that widgets respond to input events and update their visual state.

use debug_server::Empty;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_interactive_slider() {
    let manifest_dir = test_e2e::manifest_dir();
    let root_dir = manifest_dir.parent().unwrap();

    // Start binary using library helper
    println!("Starting test-e2e...");
    let mut child =
        test_e2e::spawn_test_e2e(root_dir, "widgets").expect("Failed to start test-e2e");

    // Wait for RPC server (lockfile)
    let lockfile_path = test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10))
        .await
        .expect("Timeout waiting for lockfile");

    let info = test_e2e::parse_lockfile(&lockfile_path).expect("Failed to parse lockfile");
    println!("RPC port found: {}", info.port);

    // Connect RPC client
    let mut client = test_e2e::connect_rpc(info.port, Duration::from_secs(5))
        .await
        .expect("Failed to connect to RPC server");

    println!("Connected to RPC server");

    // Wait for app to be ready
    client
        .wait_for_app_ready(Empty {})
        .await
        .expect("Failed to wait for app ready");

    // --- TEST 1: Slider Interaction via Input Events ---
    println!("--- TEST 1: Slider Interaction ---");
    let slider_id = 3;

    // Query initial state
    let initial_state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: slider_id as u64,
        })
        .await
        .expect("Failed to get widget state")
        .into_inner();

    let initial_slider_state = match initial_state.specific_state {
        Some(debug_server::debug_control::widget_state::SpecificState::Slider(s)) => s,
        _ => panic!("Expected Slider state"),
    };

    // Capture initial screenshot
    let initial_img_bytes = client
        .capture_screen(debug_server::debug_control::Empty {})
        .await
        .expect("Failed to capture screen")
        .into_inner();

    // Simulate interaction: Click at 0.0 (left edge of slider)
    println!("Sending MouseClick...");
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 60.0,
                    y: 135.0,
                    button: 0,
                },
            )),
        })
        .await
        .expect("Failed to send MouseDown");

    sleep(Duration::from_millis(100)).await;

    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Up as i32,
                    x: 60.0,
                    y: 135.0,
                    button: 0,
                },
            )),
        })
        .await
        .expect("Failed to send MouseUp");

    sleep(Duration::from_millis(500)).await;

    // Query final state
    let final_state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: slider_id as u64,
        })
        .await
        .expect("Failed to get widget state")
        .into_inner();

    let final_slider_state = match final_state.specific_state {
        Some(debug_server::debug_control::widget_state::SpecificState::Slider(s)) => s,
        _ => panic!("Expected Slider state"),
    };

    assert!(
        (final_slider_state.value - 0.04).abs() < 0.05,
        "Slider value mismatch. Expected ~0.04, got {}",
        final_slider_state.value
    );
    assert_ne!(
        initial_slider_state.value, final_slider_state.value,
        "Slider value should have changed"
    );

    // Capture final screenshot
    let final_img_bytes = client
        .capture_screen(debug_server::debug_control::Empty {})
        .await
        .expect("Failed to capture screen")
        .into_inner();

    assert_ne!(
        initial_img_bytes.data, final_img_bytes.data,
        "Screenshots should differ"
    );
    println!("Slider interaction test passed!");

    // --- TEST 2: SetWidgetValue verification (Knob) ---
    println!("--- TEST 2: SetWidgetValue RPC (Knob) ---");
    // Knob 1 is widget at index 5 (Row 1: 3, Row 2: 2, Row 3 Item 1)
    // Actually:
    // Row 1: 3 Buttons (0, 1, 2)
    // Row 2: 2 Sliders (3, 4)
    // Row 3: 3 Knobs (5, 6, 7)
    let knob_id = 5;

    // Set value to 0.1
    let target_value = 0.1;
    let _ack = client
        .set_widget_value(debug_server::debug_control::SetWidgetValueMsg {
            widget_id: knob_id,
            value: target_value,
        })
        .await
        .expect("Failed to set widget value")
        .into_inner();

    sleep(Duration::from_millis(200)).await;

    // Verify state
    let knob_state_response = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: knob_id as u64,
        })
        .await
        .expect("Failed to get knob state")
        .into_inner();

    let knob_state = match knob_state_response.specific_state {
        Some(debug_server::debug_control::widget_state::SpecificState::Knob(k)) => k,
        _ => panic!("Expected Knob state"),
    };

    assert!(
        (knob_state.value - target_value).abs() < 0.001,
        "Knob value mismatch. Expected {}, got {}",
        target_value,
        knob_state.value
    );
    println!("Knob SetWidgetValue test passed!");

    // Clean exit
    let _ = client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Quit(
                debug_server::debug_control::QuitMsg {},
            )),
        })
        .await;

    child.kill().ok();
}

#[tokio::test]
async fn test_drag_capture() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    // Start binary
    let mut child =
        test_e2e::spawn_test_e2e(root_dir, "widgets").expect("Failed to start test-e2e");

    // Wait for start...
    let lockfile_path = test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10))
        .await
        .expect("Timeout waiting for lockfile");

    let info = test_e2e::parse_lockfile(&lockfile_path).expect("Failed to parse lockfile");

    let mut client = test_e2e::connect_rpc(info.port, Duration::from_secs(5))
        .await
        .expect("Failed to connect");

    // Wait for app to be ready
    client
        .wait_for_app_ready(Empty {})
        .await
        .expect("Failed to wait for app ready");

    // Test Slider Drag Capture
    // Slider 3 (Horizontal) is at roughly 50, 120, size 250x30
    // We will click in the middle, then drag WAY down (outside widget)
    // The value should still update

    println!("--- TEST 3: Slider Drag Capture ---");
    let slider_id = 3;

    // 1. Mouse Down in middle of slider (approx 175, 135)
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 175.0,
                    y: 135.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // 2. Drag to the right AND down (outside bounds)
    // Slider is y=120..150. We'll drag to y=300
    // x movement should still change value
    for i in 0..10 {
        client
            .send_input_event(debug_server::debug_control::InputEventMsg {
                event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                    debug_server::debug_control::MouseMsg {
                        r#type: debug_server::debug_control::mouse_msg::Type::Move as i32,
                        x: 175.0 + (i as f32 * 10.0), // Move right
                        y: 135.0 + (i as f32 * 20.0), // Move down (OUTSIDE)
                        button: 0,
                    },
                )),
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(10)).await;
    }

    // Check value - it should have increased significantly
    let state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: slider_id,
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(debug_server::debug_control::widget_state::SpecificState::Slider(s)) =
        state.specific_state
    {
        println!("Final slider value: {}", s.value);
        assert!(
            s.value > 0.6,
            "Slider value should increase despite mouse being outside bounds. Got {}",
            s.value
        );
    } else {
        panic!("Expected slider state");
    }

    // Release mouse from slider
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Up as i32,
                    x: 275.0,
                    y: 335.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // --- TEST 4: Knob Drag Capture ---
    println!("--- TEST 4: Knob Drag Capture ---");
    let knob_id = 5; // Knob 1 center approx (90, 295)
    let center_x = 90.0;
    let center_y = 295.0;

    // 1. Mouse Down on Knob
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: center_x,
                    y: center_y,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // 2. Drag Up and Left (outside bounds)
    // Knob is y=280..310. We'll drag to y=200 (up = increase value)
    for i in 0..10 {
        client
            .send_input_event(debug_server::debug_control::InputEventMsg {
                event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                    debug_server::debug_control::MouseMsg {
                        r#type: debug_server::debug_control::mouse_msg::Type::Move as i32,
                        // Use f32 for proto compatibility
                        x: center_x - (i as f32 * 5.0),  // Move left
                        y: center_y - (i as f32 * 10.0), // Move Up (OUTSIDE)
                        button: 0,
                    },
                )),
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(10)).await;
    }

    // Check value
    let state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: knob_id as u64,
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(debug_server::debug_control::widget_state::SpecificState::Knob(k)) =
        state.specific_state
    {
        println!("Final knob value: {}", k.value);
        assert!(
            k.value > 0.4,
            "Knob value should increase (drag up). Got {}",
            k.value
        );
    } else {
        panic!("Expected knob state");
    }

    // Clean exit
    let _ = client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Quit(
                debug_server::debug_control::QuitMsg {},
            )),
        })
        .await;

    child.kill().ok();
}
