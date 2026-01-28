use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use testlib;

/// Run test-e2e to generate golden images
pub fn generate() -> Result<()> {
    println!("Generating golden images...");
    let root = project_root();
    let goldens_dir = root.join("test-e2e/goldens");
    std::fs::create_dir_all(&goldens_dir)?;

    println!("Building test-e2e...");
    let build_status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "-p", "test-e2e"])
        .status()
        .context("Failed to build test-e2e")?;

    if !build_status.success() {
        anyhow::bail!("Failed to build test-e2e");
    }

    // Create tokio runtime
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create tokio runtime")?;

    // Generate widget showcase golden
    println!("\n=== Generating Widget Showcase Golden ===");
    let widget_img =
        rt.block_on(async { testlib::capture_test_e2e_golden(&root, "widgets").await })?;

    let widget_golden_path = goldens_dir.join("widgets_showcase.png");
    widget_img
        .save(&widget_golden_path)
        .context("Failed to save widget golden")?;
    println!("Widget golden saved to {}", widget_golden_path.display());

    // Also generate standalone golden (for backwards compatibility)
    println!("\n=== Generating Standalone Golden ===");
    println!("Building standalone...");
    let standalone_status = Command::new("cargo")
        .current_dir(&root)
        .args(["build", "-p", "standalone"])
        .status()
        .context("Failed to build standalone")?;

    if !standalone_status.success() {
        anyhow::bail!("Failed to build standalone");
    }

    let standalone_img = rt.block_on(async { testlib::capture_golden(&root).await })?;

    let standalone_golden_path = goldens_dir.join("standalone_screenshot.png");
    standalone_img
        .save(&standalone_golden_path)
        .context("Failed to save standalone golden")?;
    println!(
        "Standalone golden saved to {}",
        standalone_golden_path.display()
    );

    println!("\nAll goldens generated successfully!");
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
