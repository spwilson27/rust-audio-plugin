use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run the standalone app to generate a screenshot, then copy it to the goldens directory.
pub fn generate() -> Result<()> {
    println!("Generating golden images...");
    let root = project_root();
    let goldens_dir = root.join("standalone/tests/goldens");
    std::fs::create_dir_all(&goldens_dir)?;

    println!("Running standalone to generate screenshot...");
    let status = Command::new("cargo")
        .current_dir(&root)
        .args(&["run", "-p", "standalone", "--", "--test-screenshot"])
        .status()
        .context("Failed to run standalone")?;

    if !status.success() {
        anyhow::bail!("Standalone application failed");
    }

    let screenshot_path = root.join("screenshot.png");
    if !screenshot_path.exists() {
        anyhow::bail!(
            "Screenshot was not generated at {}",
            screenshot_path.display()
        );
    }

    let golden_path = goldens_dir.join("standalone_screenshot.png");
    std::fs::copy(&screenshot_path, &golden_path)?;
    println!("Golden image updated at {}", golden_path.display());

    // Cleanup
    let _ = std::fs::remove_file(screenshot_path);
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf()
}
