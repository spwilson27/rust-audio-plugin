use anyhow::Result;
use debug_server::{InputEventMsg, MouseMsg};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn test_rpc_event_injection() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    // 1. Launch in headless mode
    println!("Launching test-e2e headless...");
    let mut child = test_e2e::spawn_test_e2e(root_dir, "headless")?;

    // 2. Wait for RPC
    println!("Waiting for lockfile...");
    let lockfile = match test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10)).await {
        Ok(l) => l,
        Err(e) => {
            child.kill().ok();
            return Err(e);
        }
    };

    let info = test_e2e::parse_lockfile(&lockfile)?;
    println!("Headless running on port {}", info.port);

    // 3. Connect Client
    println!("Connecting RPC...");
    let mut client = match test_e2e::connect_rpc(info.port, Duration::from_secs(5)).await {
        Ok(c) => c,
        Err(e) => {
            child.kill().ok();
            return Err(e);
        }
    };

    // 4. Send Event
    println!("Sending MouseDown...");
    let request = tonic::Request::new(InputEventMsg {
        event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
            MouseMsg {
                r#type: 0, // Down
                x: 123.0,
                y: 456.0,
                button: 0,
            },
        )),
    });

    let response = client.send_input_event(request).await?;
    let inner = response.into_inner();
    assert!(inner.success, "RPC server returned failure");

    // Cleanup
    test_e2e::quit_process(&mut client).await.ok();
    child.kill().ok();

    Ok(())
}
