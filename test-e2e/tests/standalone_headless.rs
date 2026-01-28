use anyhow::Result;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

#[tokio::test]
async fn test_standalone_headless_debug() -> Result<()> {
    // 1. Spawn standalone --headless --debug-server
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_dir = manifest_dir.parent().unwrap();

    testlib::cleanup_lockfiles()?;

    println!("Spawning standalone headless debug...");
    let mut child = Command::new("cargo")
        .current_dir(root_dir)
        .args([
            "run",
            "-p",
            "standalone",
            "--",
            "--headless",
            "--debug-server",
        ])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    // 2. Wait for RPC
    let lockfile = match testlib::wait_for_lockfile(Duration::from_secs(10)).await {
        Ok(l) => l,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    let info = testlib::parse_lockfile(&lockfile)?;
    println!("Connected on port {}", info.port);

    let mut client = match testlib::connect_rpc(info.port, Duration::from_secs(5)).await {
        Ok(c) => c,
        Err(e) => {
            let _ = child.kill();
            return Err(e);
        }
    };

    // 3. Send Quit
    println!("Sending Quit...");
    if let Err(e) = testlib::quit_standalone(&mut client).await {
        let _ = child.kill();
        return Err(e);
    }

    // 4. Verify exit
    let status = child.wait()?;
    assert!(status.success(), "Standalone failed to exit cleanly");

    Ok(())
}
