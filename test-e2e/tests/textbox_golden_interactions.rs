//! Textbox Golden Interactions
//!
//! Verifies rendering of Textbox during interactions (focus, type, select, edit).
//! Uses "--mode text-resize" which provides a single Textbox at (50, 50, 300, 40)
//! with text "Resize Test Mode".

use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_textbox_interactions_golden() {
    let manifest_dir = test_e2e::manifest_dir();
    let root_dir = manifest_dir.parent().unwrap();
    let goldens_dir = manifest_dir.join("goldens");
    std::fs::create_dir_all(&goldens_dir).unwrap();

    // 1. Start App
    let mut tester = test_e2e::GoldenTester::new();
    let mut child = test_e2e::spawn_test_e2e(root_dir, "text-resize").expect("Failed to start app");

    // Connect RPC
    let lockfile_path = test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10))
        .await
        .expect("Lockfile timeout");

    let info = test_e2e::parse_lockfile(&lockfile_path).expect("Failed to parse lockfile");
    let mut client = test_e2e::connect_rpc(info.port, Duration::from_secs(5))
        .await
        .expect("RPC connection failed");

    // Wait for initial render
    sleep(Duration::from_millis(500)).await;

    // --- SNAPSHOT 1: Initial ---
    {
        println!("Capturing Initial State...");
        let img = test_e2e::capture_screenshot(&mut client)
            .await
            .expect("Capture failed");
        tester.check(&img, &goldens_dir.join("textbox_1_initial.png"));
    }

    // --- Interaction: Click to Focus ---
    // Textbox is at 50,50. Click inside at 100,60.
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 100.0,
                    y: 60.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Up as i32,
                    x: 100.0,
                    y: 60.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // Wait for cursor blink/render
    sleep(Duration::from_millis(500)).await;

    // --- SNAPSHOT 2: Focused ---
    {
        println!("Capturing Focused State...");
        let img = test_e2e::capture_screenshot(&mut client)
            .await
            .expect("Capture failed");
        tester.check(&img, &goldens_dir.join("textbox_2_focused.png"));
    }

    // --- Interaction: Type " 123" ---
    for c in " 123".chars() {
        client
            .send_input_event(debug_server::debug_control::InputEventMsg {
                event: Some(
                    debug_server::debug_control::input_event_msg::Event::TextInput(
                        debug_server::debug_control::TextInputMsg {
                            text: c.to_string(),
                        },
                    ),
                ),
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(50)).await;
    }

    // Wait for render
    sleep(Duration::from_millis(750)).await;

    // --- SNAPSHOT 3: Typed ---
    {
        println!("Capturing Typed State...");
        let img = test_e2e::capture_screenshot(&mut client)
            .await
            .expect("Capture failed");
        tester.check(&img, &goldens_dir.join("textbox_3_typed.png"));
    }

    // --- Interaction: Drag Select "Test" ---
    // Text: "Resize Test Mode 123"
    // "Resize " is approx 50-100px?
    // Let's drag from x=100 to x=200.
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 100.0,
                    y: 60.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    for i in 0..10 {
        client
            .send_input_event(debug_server::debug_control::InputEventMsg {
                event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                    debug_server::debug_control::MouseMsg {
                        r#type: debug_server::debug_control::mouse_msg::Type::Move as i32,
                        x: 100.0 + (i as f32 * 10.0), // 100 -> 200
                        y: 60.0,
                        button: 0,
                    },
                )),
            })
            .await
            .unwrap();
        sleep(Duration::from_millis(20)).await;
    }

    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Up as i32,
                    x: 200.0,
                    y: 60.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // Wait for render
    sleep(Duration::from_millis(500)).await;

    // --- SNAPSHOT 4: Selected ---
    {
        println!("Capturing Selected State...");
        let img = test_e2e::capture_screenshot(&mut client)
            .await
            .expect("Capture failed");
        tester.check(&img, &goldens_dir.join("textbox_4_selected.png"));
    }

    // --- Interaction: Backspace (Delete Selection) ---
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Key(
                debug_server::debug_control::KeyMsg {
                    r#type: debug_server::debug_control::key_msg::Type::Down as i32,
                    keycode: 51, // Backspace
                    modifiers: 0,
                },
            )),
        })
        .await
        .unwrap();

    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Key(
                debug_server::debug_control::KeyMsg {
                    r#type: debug_server::debug_control::key_msg::Type::Up as i32,
                    keycode: 51,
                    modifiers: 0,
                },
            )),
        })
        .await
        .unwrap();

    // Wait for render
    sleep(Duration::from_millis(500)).await;

    // --- SNAPSHOT 5: Edited ---
    {
        println!("Capturing Edited State...");
        let img = test_e2e::capture_screenshot(&mut client)
            .await
            .expect("Capture failed");
        tester.check(&img, &goldens_dir.join("textbox_5_edited.png"));
    }

    // Cleanup
    test_e2e::quit_process(&mut client).await.ok();
    child.kill().ok();
    tester.assert();
}
