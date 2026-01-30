use std::path::PathBuf;
use std::process::Command;
use anyhow::{bail, Result};

#[test]
fn validate_vst3_with_pluginval() -> Result<()> {
    // 1. Ensure pluginval is available
    let pluginval_path = get_pluginval_path()?;
    if !pluginval_path.exists() {
        println!("pluginval not found at {:?}, skipping test.", pluginval_path);
        println!("Run 'mkdir -p extern/pluginval && ...' to install it.");
        // We typically don't want to fail CI if an optional tool is missing, 
        // unless we enforce it installing. 
        // For this task, I'll assume it's installed or fail if not to be strict.
        bail!("pluginval binary not found at {:?}", pluginval_path);
    }

    // 2. Build the plugin (VST3)
    // We assume 'cargo xtask bundle' has been run or we run it now.
    // Running it here ensures fresh build.
    let root = project_root()?;
    let status = Command::new("cargo")
        .args(["xtask", "bundle", "--format", "vst3"])
        .current_dir(&root)
        .status()?;
    
    if !status.success() {
        bail!("Failed to bundle VST3 plugin");
    }

    // 3. Path to bundle
    let bundle_path = root.join("target/bundled/splug.vst3");
    if !bundle_path.exists() {
        bail!("VST3 bundle not found at {:?}", bundle_path);
    }

    // 4. Run pluginval
    println!("Running pluginval...");
    let status = Command::new(&pluginval_path)
        .arg("--validate")
        .arg(&bundle_path)
        .arg("--strict")
        .arg("--verbose")
        .status()?;

    if !status.success() {
        bail!("pluginval validation failed");
    }

    Ok(())
}

fn get_pluginval_path() -> Result<PathBuf> {
    let root = project_root()?;
    
    // Prefer locally built debug version
    #[cfg(target_os = "macos")]
    let debug_path = root.join("target/pluginval-debug/pluginval_artefacts/Debug/pluginval.app/Contents/MacOS/pluginval");
    #[cfg(target_os = "macos")]
    let release_path = root.join("extern/pluginval/pluginval.app/Contents/MacOS/pluginval");

    if debug_path.exists() {
        return Ok(debug_path);
    }
    
    Ok(release_path)
}

fn project_root() -> Result<PathBuf> {
    let output = Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()?;
    
    let path = String::from_utf8(output.stdout)?.trim().to_string();
    let path = PathBuf::from(path);
    Ok(path.parent().unwrap().to_path_buf())
}
