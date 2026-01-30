//! Textbox Shortcuts E2E Tests
//!
//! Verifies OS shortcuts like Copy, Paste, Select All, Delete Word.

use debug_server::DebugControlClient;
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
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn setup_test() -> (ProcessGuard, DebugControlClient<tonic::transport::Channel>) {
    let manifest_dir = test_e2e::manifest_dir();
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
async fn test_select_all_and_replace() {
    let (mut _child, mut client): (ProcessGuard, DebugControlClient<tonic::transport::Channel>) =
        setup_test().await;

    // Use Textbox ID 8 ("Sample Text")
    let textbox_id = 8;

    // Focus via click
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 60.0,
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

    // Verify initial text
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
        assert_eq!(tb.text, "Sample Text");
    }

    // Send Cmd+A (Select All)
    // pal::Modifiers::META = 1 << 3 = 8
    println!("Sending Cmd+A...");
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Key(
                debug_server::debug_control::KeyMsg {
                    r#type: debug_server::debug_control::key_msg::Type::Down as i32,
                    keycode: 0,   // 'A' keycode on macOS
                    modifiers: 8, // META
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
                    keycode: 0,
                    modifiers: 8,
                },
            )),
        })
        .await
        .unwrap();

    // Verify selection (Implicitly checked by replacement)

    // Type "All Gone"
    println!("Typing replacement...");
    for c in "All Gone".chars() {
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

    // Verify final text
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
        println!("Final text: '{}'", tb.text);
        assert_eq!(tb.text, "All Gone");
    }
}

#[tokio::test]
async fn test_delete_word_backward() {
    let (mut _child, mut client): (ProcessGuard, DebugControlClient<tonic::transport::Channel>) =
        setup_test().await;
    let textbox_id = 10; // "Focused"

    // Focus
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
                debug_server::debug_control::MouseMsg {
                    r#type: debug_server::debug_control::mouse_msg::Type::Down as i32,
                    x: 240.0,
                    y: 460.0, // ID 10 is at y=450
                    // Wait, textbox_e2e used ID 10. `main.rs` defines IDs.
                    // ID 10 is "Focused".
                    // Let's assume layout hasn't changed.
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
                    x: 240.0,
                    y: 460.0,
                    button: 0,
                },
            )),
        })
        .await
        .unwrap();

    // Text is "Focused"
    // Send Input " Word" -> "Focused Word"
    for c in " Word".chars() {
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

    // Send Option+Backspace
    // Modifiers::ALT = 1 << 2 = 4
    // Backspace = 51
    println!("Sending Option+Backspace...");
    client
        .send_input_event(debug_server::debug_control::InputEventMsg {
            event: Some(debug_server::debug_control::input_event_msg::Event::Key(
                debug_server::debug_control::KeyMsg {
                    r#type: debug_server::debug_control::key_msg::Type::Down as i32,
                    keycode: 51,
                    modifiers: 4,
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
                    modifiers: 4,
                },
            )),
        })
        .await
        .unwrap();

    // Verify text: "Focused " (Word deleted)
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
        println!("Text after delete word: '{}'", tb.text);
        assert_eq!(tb.text, "Focused ", "Should have deleted 'Word'");
    }
}
