use anyhow::Result;
use debug_server::debug_control::{
    input_event_msg::Event, InputEventMsg, KeyMsg, MouseMsg, QuitMsg, ResizeMsg,
};
use std::path::Path;
use std::time::Duration;
use test_e2e::*;

#[tokio::test]
async fn test_audio_selection_ui() -> Result<()> {
    // 1. Spawn Example
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let guard = spawn_example(&root, "audio_selection", &["--debug-server"])?;

    // 2. Connect RPC
    let lock_path = wait_for_lockfile(guard.id(), Duration::from_secs(10)).await?;
    let info = parse_lockfile(&lock_path)?;
    let mut client = connect_rpc(info.port, Duration::from_secs(5)).await?;

    // 3. Ensure Window Size (600x400)
    client
        .resize_window(ResizeMsg {
            width: 600,
            height: 400,
        })
        .await?;

    // Wait for layout to settle
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 4. Initial State (Collapsed)
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(&img, Path::new("tests/goldens/audio_selection_initial.png"));
    }

    // 5. Open Dropdown
    // Click center of dropdown: (300, 200)
    // Mouse Move
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 110.0,
                button: 0,
                r#type: 2, // Move
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Mouse Down (Left)
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 110.0,
                button: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Mouse Up
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 110.0,
                button: 0,
                r#type: 1, // Up
            })),
        })
        .await?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    // State: Expanded
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(
            &img,
            Path::new("tests/goldens/audio_selection_expanded.png"),
        );
    }

    // 6. Hover Item 2
    // Item 1 Y: 220-250. Center 235.
    // Item 2 Y: 250-280. Center 265.
    // Move to (300, 265)
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 175.0,
                button: 0,
                r#type: 2, // Move
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // State: Hover Item 2
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(&img, Path::new("tests/goldens/audio_selection_hover.png"));
    }

    // 7. Click Item 2 (Select)
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 175.0,
                button: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 175.0,
                button: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // State: Selected (Collapsed, New Value)
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(
            &img,
            Path::new("tests/goldens/audio_selection_selected.png"),
        );
    }

    // 8. Open Dropdown again and Click Outside
    // Open
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 110.0,
                button: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 110.0,
                button: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Click Outside (10, 10)
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 10.0,
                y: 10.0,
                button: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 10.0,
                y: 10.0,
                button: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // State: Should be same as "Selected" (Still Item 2, Closed)
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(
            &img,
            Path::new("tests/goldens/audio_selection_selected_unfocused.png"),
        );
    }

    // 9. Keyboard Navigation
    // Open Dropdown
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 110.0,
                button: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 300.0,
                y: 200.0,
                button: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Press Arrow Up (126)
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Key(KeyMsg {
                keycode: 126,
                modifiers: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Key(KeyMsg {
                keycode: 126,
                modifiers: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify Highlight moved to Item 1
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(
            &img,
            Path::new("tests/goldens/audio_selection_keyboard_up.png"),
        );
    }

    // Press Enter (36)
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Key(KeyMsg {
                keycode: 36,
                modifiers: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Key(KeyMsg {
                keycode: 36,
                modifiers: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Click Outside to clear focus
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 10.0,
                y: 10.0,
                button: 0,
                r#type: 0, // Down
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Mouse(MouseMsg {
                x: 10.0,
                y: 10.0,
                button: 0,
                r#type: 1, // Up
            })),
        })
        .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;

    // State: Closed, Item 1 selected (Initial State)
    {
        let img = capture_screenshot(&mut client).await?;
        verify_golden(&img, Path::new("tests/goldens/audio_selection_initial.png"));
    }

    // Quit
    client
        .send_input_event(InputEventMsg {
            event: Some(Event::Quit(QuitMsg {})),
        })
        .await?;

    Ok(())
}
