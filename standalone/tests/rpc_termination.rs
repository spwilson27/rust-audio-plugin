use anyhow::{Context, Result};
use debug_server::DebugControlClient;
use debug_server::{InputEventMsg, QuitMsg};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct StandaloneProcess {
    child: Child,
    lockfile_path: std::path::PathBuf,
}

impl StandaloneProcess {
    fn launch() -> Result<Self> {
        // Find the binary
        let cargo_bin = env!("CARGO_BIN_EXE_standalone");

        // Launch standalone in headless mode
        let mut child = Command::new(cargo_bin)
            .arg("--headless")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn standalone")?;

        // Wait for lockfile
        let pid = child.id();
        let temp_dir = std::env::temp_dir();
        let lockfile_path = temp_dir.join(format!("splug_pid_{}.json", pid));

        let start = std::time::Instant::now();
        while !lockfile_path.exists() {
            if start.elapsed() > Duration::from_secs(10) {
                let _ = child.kill();
                anyhow::bail!("Timed out waiting for lockfile");
            }
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(Self {
            child,
            lockfile_path,
        })
    }

    fn get_port(&self) -> Result<u16> {
        let content = std::fs::read_to_string(&self.lockfile_path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;
        let port = json["port"].as_u64().context("Port not found")? as u16;
        Ok(port)
    }
}

impl Drop for StandaloneProcess {
    fn drop(&mut self) {
        // Ensure cleanup if test panics, though we expect it to exit gracefully
        let _ = self.child.kill();
        let _ = std::fs::remove_file(&self.lockfile_path);
    }
}

#[tokio::test]
async fn test_rpc_termination() -> Result<()> {
    // 1. Launch Standalone
    let mut process = StandaloneProcess::launch()?;
    let port = process.get_port()?;
    println!("Standalone running on port {}", port);

    // 2. Connect RPC Client
    let addr = format!("http://127.0.0.1:{}", port);
    let mut client = DebugControlClient::connect(addr).await?;

    // 3. Send Quit Event
    println!("Sending Quit...");
    let request = tonic::Request::new(InputEventMsg {
        event: Some(debug_server::debug_control::input_event_msg::Event::Quit(
            QuitMsg {},
        )),
    });

    let response = client.send_input_event(request).await?;
    let inner = response.into_inner();
    assert!(inner.success, "RPC server returned failure");

    // 4. Verify Termination
    // Wait for the process to exit
    let start = std::time::Instant::now();
    let mut exited = false;
    while start.elapsed() < Duration::from_secs(5) {
        match process.child.try_wait() {
            Ok(Some(status)) => {
                println!("Process exited with: {}", status);
                exited = true;
                break;
            }
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => anyhow::bail!("Error checking process status: {}", e),
        }
    }

    assert!(exited, "Standalone process did not exit within timeout");

    Ok(())
}
