//! Interactive E2E test for widgets
//!
//! Verifies that widgets respond to input events and update their visual state.

use debug_server::DebugControlClient;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tonic::transport::Endpoint;

/// RAII wrapper to ensure child process is killed
struct ProcessGuard(Child);

impl ProcessGuard {
    fn spawn(bin_path: &str, args: &[&str]) -> anyhow::Result<Self> {
        let child = Command::new(bin_path)
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;
        Ok(Self(child))
    }

    fn id(&self) -> u32 {
        self.0.id()
    }

    fn kill(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn test_interactive_slider() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    // Build test-e2e first
    println!("Building test-e2e...");
    let status = Command::new("cargo")
        .current_dir(root_dir)
        .args(["build", "-p", "test-e2e"])
        .status()
        .expect("Failed to build test-e2e");
    assert!(status.success(), "Failed to build test-e2e");

    // Start binary
    println!("Starting test-e2e...");
    let mut child = ProcessGuard::spawn(
        root_dir.join("target/debug/test-e2e").to_str().unwrap(),
        &["--mode", "widgets", "--fixed-fps"],
    )
    .expect("Failed to start test-e2e");

    // Wait for RPC server to start (lockfile)
    let pid = child.id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("test_e2e_pid_{}.json", pid));

    println!("Waiting for lockfile at: {}", lockfile_path.display());
    let mut port = 0;
    for _ in 0..100 {
        if lockfile_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&lockfile_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(p) = json.get("port").and_then(|v| v.as_u64()) {
                        port = p as u16;
                        break;
                    }
                }
            }
        }
        sleep(Duration::from_millis(100)).await;
    }

    assert!(port > 0, "Failed to find RPC port");
    println!("RPC port found: {}", port);

    // Connect RPC client with increased limits
    let addr = format!("http://127.0.0.1:{}", port);
    let channel = Endpoint::from_shared(addr)
        .expect("Invalid URI")
        .connect()
        .await
        .expect("Failed to connect to RPC server");

    let mut client = DebugControlClient::new(channel)
        .max_decoding_message_size(16 * 1024 * 1024)
        .max_encoding_message_size(16 * 1024 * 1024);

    println!("Connected to RPC server");

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

    child.kill();
}
