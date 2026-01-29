//! Textbox E2E Tests
//!
//! Verifies advanced text editing features via RPC.

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

async fn setup_test() -> (ProcessGuard, DebugControlClient<tonic::transport::Channel>) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    // Start binary
    let child = ProcessGuard::spawn(
        root_dir.join("target/debug/test-e2e").to_str().unwrap(),
        &["--mode", "widgets", "--fixed-fps"],
    )
    .expect("Failed to start test-e2e");

    // Wait for RPC server (lockfile)
    let pid = child.id();
    let temp_dir = std::env::temp_dir();
    let lockfile_path = temp_dir.join(format!("test_e2e_pid_{}.json", pid));

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

    let addr = format!("http://127.0.0.1:{}", port);
    let channel = Endpoint::from_shared(addr)
        .expect("Invalid URI")
        .connect()
        .await
        .expect("Failed to connect to RPC server");

    let client = DebugControlClient::new(channel)
        .max_decoding_message_size(16 * 1024 * 1024)
        .max_encoding_message_size(16 * 1024 * 1024);

    (child, client)
}

#[tokio::test]
async fn test_text_entry_and_deletion() {
    let (mut _child, mut client) = setup_test().await;

    // Use correct ID for the focused Textbox (ID 10 from main.rs)
    let textbox_id = 10;

    // 1. Initial State
    // "Focused" is the initial text
    let state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: textbox_id as u64,
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(debug_server::debug_control::widget_state::SpecificState::Textbox(tb)) =
        state.specific_state
    {
        assert_eq!(tb.text, "Focused");
    } else {
        panic!("Expected Textbox state");
    }

    // 2. Type " World"
    println!("Sending characters...");
    let chars = " World";
    for c in chars.chars() {
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

    // Verify text: "Focused World"
    let state_mid = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: textbox_id as u64,
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(debug_server::debug_control::widget_state::SpecificState::Textbox(tb)) =
        state_mid.specific_state
    {
        println!("Text after typing: '{}'", tb.text);
        assert_eq!(tb.text, "Focused World");
    }

    // 3. Backspace 6 times (Delete " World")
    println!("Sending Backspaces...");
    for _ in 0..6 {
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
    }

    // Verify text: "Focused"
    let state_final = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: textbox_id as u64,
        })
        .await
        .unwrap()
        .into_inner();

    if let Some(debug_server::debug_control::widget_state::SpecificState::Textbox(tb)) =
        state_final.specific_state
    {
        println!("Text after backspace: '{}'", tb.text);
        assert_eq!(tb.text, "Focused");
    }

    _child.0.kill().unwrap();
}

#[tokio::test]
async fn test_drag_selection_and_replace() {
    let (mut child, mut client) = setup_test().await;

    // main.rs: textbox_text ("Sample Text") is ID 8.
    // Let's use ID 8.
    let textbox_id = 8;

    // Focus it first
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 60.0, // Inside 50,380 200x35
                    y: 390.0,
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
                    x: 60.0,
                    y: 390.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // Verify Text: "Sample Text"
    // Drag select "Sample "
    // Start at x=50+5 (approx) -> "S"
    // Drag to x=50+5+text_width("Sample ")
    // Rough estimate: 16px font * 6 chars ~ 96px?

    // MouseDown at start
    println!("Starting drag selection...");
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 55.0, // Start of text
                    y: 390.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // MouseMove to right
    for i in 0..10 {
        client
            .send_input_event(debug_server::debug_control::InputEventMsg {
                event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                    debug_server::debug_control::MouseMsg {
                        r#type: debug_server::debug_control::mouse_msg::Type::Move as i32,
                        x: 55.0 + (i as f32 * 10.0), // Move 100px right
                        y: 390.0,
                        button: 0,
                    },
                )),
            })
            .await
            .unwrap();
    }

    // MouseUp
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Up as i32,
                    x: 155.0,
                    y: 390.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // Verify selection exists
    let state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: textbox_id,
        })
        .await
        .unwrap()
        .into_inner();
    if let Some(debug_server::debug_control::widget_state::SpecificState::Textbox(tb)) =
        state.specific_state
    {
        assert!(tb.has_selection, "Should have selection after drag");
    }

    // Type "New" -> Should replace selection
    println!("Replacing selection...");
    for c in "New".chars() {
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
    }

    // Verify text: "NewText" or similar (depending on how much "Sample " was selected)
    // "Sample Text" -> "New" + remainder
    // We don't know exact pixel width of "Sample ", but we dragged 100px.
    // "Sample " is likely selected.

    let state_final = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: textbox_id,
        })
        .await
        .unwrap()
        .into_inner();
    if let Some(debug_server::debug_control::widget_state::SpecificState::Textbox(tb)) =
        state_final.specific_state
    {
        println!("Final text: '{}'", tb.text);
        assert!(
            tb.text.starts_with("New"),
            "Text should start with replacement"
        );
        assert_ne!(tb.text, "Sample Text", "Text should have changed");
    }

    child.kill();
}

#[tokio::test]
async fn test_double_click_selection() {
    let (mut child, mut client) = setup_test().await;
    let textbox_id = 8; // "Sample Text"

    // Double click on "Text" (last word)
    // "Sample " ~ 7 chars * 10px? + padding 5.
    // Let's guess x=120.0

    let click_x = 150.0;
    let click_y = 390.0;

    // TODO We should have an RPC to wait for the server to come up
    sleep(Duration::from_millis(200)).await; // Wait for the server to come up??

    // Click 1
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: click_x,
                    y: click_y,
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
                    x: click_x,
                    y: click_y,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    sleep(Duration::from_millis(100)).await; // Fast enough for double click (<500ms)

    // Click 2
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: click_x,
                    y: click_y,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    sleep(Duration::from_millis(200)).await; // Wait for event processing

    // State check immediate after Down?
    // Logic sets selection on Down for double click.

    let state = client
        .get_widget_state(debug_server::debug_control::WidgetIdMsg {
            widget_id: textbox_id,
        })
        .await
        .unwrap()
        .into_inner();
    if let Some(debug_server::debug_control::widget_state::SpecificState::Textbox(tb)) =
        state.specific_state
    {
        assert!(tb.has_selection, "Should select word on double click");
    }

    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Up as i32,
                    x: click_x,
                    y: click_y,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    child.kill();
}
