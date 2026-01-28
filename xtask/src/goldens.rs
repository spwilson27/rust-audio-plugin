use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use testlib;

/// Run the standalone app to generate a screenshot via RPC, then copy it to the goldens directory.
pub fn generate() -> Result<()> {
    println!("Generating golden images...");
    let root = project_root();
    let goldens_dir = root.join("standalone/tests/goldens");
    std::fs::create_dir_all(&goldens_dir)?;

    println!("Building standalone...");
    let build_status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "-p", "standalone"])
        .status()
        .context("Failed to build standalone")?;

    if !build_status.success() {
        anyhow::bail!("Failed to build standalone");
    }

    // Use testlib to capture golden image
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    let img = rt.block_on(async { testlib::capture_golden(&root).await })?;

    // Save golden image
    let golden_path = goldens_dir.join("standalone_screenshot.png");
    img.save(&golden_path)
        .context("Failed to save golden image")?;
    println!("Golden image saved to {}", golden_path.display());

    // Also generate resize test golden
    println!("Running text_resize_e2e test to generate golden...");
    let status = Command::new("cargo")
        .current_dir(&root)
        // Set env var to tell test to update golden
        .env("UPDATE_GOLDENS", "1")
        .args(["test", "--test", "text_resize_e2e", "--", "--nocapture"])
        .status()
        .context("Failed to run text_resize_e2e test")?;

    if !status.success() {
        anyhow::bail!("text_resize_e2e generation failed");
    }

    println!("All goldens generated successfully.");
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
