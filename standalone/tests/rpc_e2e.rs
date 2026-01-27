use anyhow::{Context, Result};
use debug_server::{DebugControlClient, InputEventMsg, MouseMsg};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::sleep;

struct StandaloneProcess {
    child: Child,
    pid_file: std::path::PathBuf,
}

impl StandaloneProcess {
    fn launch() -> Result<Self> {
        // Find the binary
        let cargo_bin = env!("CARGO_BIN_EXE_standalone");

        let child = Command::new(cargo_bin)
            .arg("--headless")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped()) // Capture stderr too
            .spawn()
            .context("Failed to spawn standalone")?;

        let pid = child.id();
        let pid_file = std::env::temp_dir().join(format!("splug_pid_{}.json", pid));

        Ok(Self { child, pid_file })
    }

    async fn wait_for_port(&self) -> Result<u16> {
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) {
            if self.pid_file.exists() {
                let content =
                    std::fs::read_to_string(&self.pid_file).context("Failed to read pid file")?;

                // Retry if file is empty/partial
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(port) = json["port"].as_u64() {
                        return Ok(port as u16);
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
        anyhow::bail!("Timeout waiting for standalone to start RPC server");
    }
}

impl Drop for StandaloneProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait(); // Prevent zombies
        if self.pid_file.exists() {
            let _ = std::fs::remove_file(&self.pid_file);
        }
    }
}

#[tokio::test]
async fn test_rpc_event_injection() -> Result<()> {
    // 1. Launch standalone
    let process = StandaloneProcess::launch()?;
    let port = process.wait_for_port().await?;
    println!("Standalone running on port {}", port);

    // 2. Connect Client
    let addr = format!("http://127.0.0.1:{}", port);
    let mut client = DebugControlClient::connect(addr)
        .await
        .context("Failed to connect client")?;

    // 3. Send Event
    println!("Sending MouseDown...");
    // Use debug_server's re-exported types if possible, or fully qualify
    // The previous error was regarding 'tonic' crate being missing.
    // Now that we added it, tonic::Request should be valid.
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

    // 4. Verify Output (Optional but good)
    // We captured stdout, we could read it to verify "Headless received event" log unless we want to assume RPC success implies routing.
    // Reading stdout from a running child is tricky synchronously if we don't want to block.
    // For now, trusting the RPC Ack is "good enough" for phase 1 of this test.
    // If we want stricter verification, we'd need to consume the pipe in a thread.

    // Let's settle for RPC Ack success for now.

    Ok(())
}
