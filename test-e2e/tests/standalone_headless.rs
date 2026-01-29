//! Standalone Headless Test
//!
//! Verifies that the standalone binary runs in headless mode and serves RPC.

use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn test_standalone_headless() -> Result<()> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _root_dir = manifest_dir.parent().unwrap();

    println!("Starting standalone headless...");
    // Use `test_e2e::ProcessGuard::spawn` manually to pass specific args
    let mut child = test_e2e::ProcessGuard::spawn(
        "cargo",
        &[
            "run",
            "-p",
            "standalone",
            "--",
            "--headless",
            "--debug-server",
        ],
    )?;

    // Wait for RPC
    println!("Waiting for lockfile...");
    let lockfile = match test_e2e::wait_for_lockfile(child.id(), Duration::from_secs(10)).await {
        Ok(l) => l,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    let info = test_e2e::parse_lockfile(&lockfile)?;
    println!("Combined lockfile check passed. Port: {}", info.port);

    // Connect
    let mut client = match test_e2e::connect_rpc(info.port, Duration::from_secs(5)).await {
        Ok(c) => c,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    // Send Quit
    println!("Sending Quit...");
    if let Err(e) = test_e2e::quit_process(&mut client).await {
        let _ = child.kill();
        return Err(e);
    }

    // We added wait() to ProcessGuard, so we can use it now.
    let status = child.wait()?;
    assert!(status.success(), "Standalone failed to exit cleanly");

    Ok(())
}
