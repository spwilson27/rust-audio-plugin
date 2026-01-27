//! Build automation for splug audio plugin
//!
//! Implements cargo-xtask pattern for polyglot build process:
//! - Rust compilation
//! - Shader compilation (GLSL → SPIR-V)
//! - Platform-specific bundle creation
//! - Codesigning (macOS)

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation for splug", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build and bundle the plugin for distribution
    Bundle {
        /// Build in release mode (optimized)
        #[arg(long, default_value_t = true)]
        release: bool,
    },
    /// Generate golden images for testing
    Goldens,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Bundle { release } => bundle(release),
        Commands::Goldens => goldens::generate(),
    }
}

mod goldens;

/// Main bundle command - orchestrates the entire build process
fn bundle(release: bool) -> Result<()> {
    println!("🔨 Building splug plugin bundle...");

    let root = project_root()?;
    let profile = if release { "release" } else { "debug" };

    // Step 1: Compile shaders
    println!("\n📦 Step 1/4: Compiling shaders...");
    compile_shaders(&root)?;

    // Step 2: Build Rust library
    println!("\n📦 Step 2/4: Building Rust library ({})...", profile);
    build_rust_library(release)?;

    // Step 3: Create platform bundle
    println!("\n📦 Step 3/4: Creating platform bundle...");
    create_bundle(&root, profile)?;

    // Step 4: Codesign (macOS only)
    #[cfg(target_os = "macos")]
    {
        println!("\n📦 Step 4/4: Codesigning bundle...");
        codesign_bundle(&root)?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        println!("\n📦 Step 4/4: Codesigning (skipped on non-macOS)");
    }

    println!("\n✅ Bundle complete!");
    print_bundle_location(&root)?;

    Ok(())
}

/// Compile GLSL shaders to SPIR-V
fn compile_shaders(root: &Path) -> Result<()> {
    let shader_dir = root.join("plugin/src/shaders");
    let output_dir = root.join("target/shaders");

    std::fs::create_dir_all(&output_dir).context("Failed to create shader output directory")?;

    if !shader_dir.exists() {
        println!("  ⚠️  No shader directory found, skipping shader compilation");
        return Ok(());
    }

    // Find glslc in PATH or common Vulkan SDK locations
    let glslc_path = find_glslc();

    if glslc_path.is_none() {
        println!("  ⚠️  glslc not found - skipping shader compilation");
        println!("      Install Vulkan SDK to enable shader compilation:");
        println!("      macOS: brew install vulkan-tools");
        println!("      Or download from: https://vulkan.lunarg.com/");
        return Ok(());
    }

    let glslc = glslc_path.unwrap();
    println!("  Using glslc: {}", glslc.display());

    let mut compiled_count = 0;

    for entry in WalkDir::new(&shader_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str());

        // Only compile .vert and .frag files
        if !matches!(ext, Some("vert") | Some("frag")) {
            continue;
        }

        let output_name = format!("{}.spv", path.file_name().unwrap().to_str().unwrap());
        let output_path = output_dir.join(output_name);

        println!("  Compiling: {}", path.display());

        let status = Command::new(&glslc)
            .arg(path)
            .arg("-o")
            .arg(&output_path)
            .status()
            .context("Failed to run glslc")?;

        if !status.success() {
            bail!("Shader compilation failed for: {}", path.display());
        }

        compiled_count += 1;
    }

    if compiled_count == 0 {
        println!("  ⚠️  No shaders found to compile");
    } else {
        println!("  ✓ Compiled {} shader(s)", compiled_count);
    }

    Ok(())
}

/// Find glslc compiler in PATH or common Vulkan SDK locations
fn find_glslc() -> Option<PathBuf> {
    // Try PATH first
    if Command::new("glslc").arg("--version").output().is_ok() {
        return Some(PathBuf::from("glslc"));
    }

    // Common Vulkan SDK installation paths
    let home = std::env::var("HOME").ok()?;
    let possible_paths = [
        // User-local installation
        format!("{}/.local/VulkanSDK", home),
        // System installations
        "/usr/local/bin".to_string(),
        "/opt/homebrew/bin".to_string(),
    ];

    for base_path in &possible_paths {
        let base = PathBuf::from(base_path);

        // For VulkanSDK directory, search for version subdirectories
        if base.join("1.4.335.1").exists()
            || base.file_name().and_then(|n| n.to_str()) == Some("VulkanSDK")
        {
            // Search for glslc in version subdirectories
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let version_dir = entry.path();
                    if version_dir.is_dir() {
                        // macOS: check macOS/bin/glslc
                        let macos_glslc = version_dir.join("macOS/bin/glslc");
                        if macos_glslc.exists() {
                            return Some(macos_glslc);
                        }
                        // Linux/Windows: check bin/glslc
                        let bin_glslc = version_dir.join("bin/glslc");
                        if bin_glslc.exists() {
                            return Some(bin_glslc);
                        }
                    }
                }
            }
        } else {
            // Direct binary path
            let glslc = base.join("glslc");
            if glslc.exists() {
                return Some(glslc);
            }
        }
    }

    None
}

/// Build the Rust library
fn build_rust_library(release: bool) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--package").arg("splug").arg("--lib");

    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("Failed to run cargo build")?;

    if !status.success() {
        bail!("Cargo build failed");
    }

    println!("  ✓ Rust library compiled");
    Ok(())
}

/// Create platform-specific bundle structure
fn create_bundle(root: &Path, profile: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        create_macos_bundle(root, profile)
    }

    #[cfg(target_os = "windows")]
    {
        create_windows_bundle(root, profile)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        bail!("Unsupported platform for bundle creation")
    }
}

#[cfg(target_os = "macos")]
fn create_macos_bundle(root: &Path, profile: &str) -> Result<()> {
    let bundle_dir = root.join("target/bundled/splug.vst3");
    let contents_dir = bundle_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    // Create bundle directory structure
    std::fs::create_dir_all(&macos_dir).context("Failed to create bundle directories")?;

    // Copy dylib
    let dylib_src = root.join(format!("target/{}/libsplug.dylib", profile));
    let dylib_dst = macos_dir.join("splug");

    if !dylib_src.exists() {
        bail!("Library not found at: {}", dylib_src.display());
    }

    std::fs::copy(&dylib_src, &dylib_dst).context("Failed to copy dylib to bundle")?;

    println!(
        "  ✓ Copied dylib: {} → {}",
        dylib_src.display(),
        dylib_dst.display()
    );

    // Generate Info.plist
    let plist_content = generate_info_plist();
    let plist_path = contents_dir.join("Info.plist");
    std::fs::write(&plist_path, plist_content).context("Failed to write Info.plist")?;

    println!("  ✓ Generated Info.plist");

    // Generate PkgInfo
    let pkginfo_path = contents_dir.join("PkgInfo");
    std::fs::write(&pkginfo_path, "BNDL????").context("Failed to write PkgInfo")?;

    println!("  ✓ Generated PkgInfo");

    Ok(())
}

#[cfg(target_os = "windows")]
fn create_windows_bundle(root: &Path, profile: &str) -> Result<()> {
    let bundle_dir = root.join("target/bundled");
    std::fs::create_dir_all(&bundle_dir).context("Failed to create bundle directory")?;

    let dll_src = root.join(format!("target/{}/splug.dll", profile));
    let dll_dst = bundle_dir.join("splug.vst3");

    if !dll_src.exists() {
        bail!("Library not found at: {}", dll_src.display());
    }

    std::fs::copy(&dll_src, &dll_dst).context("Failed to copy dll to bundle")?;

    println!(
        "  ✓ Copied DLL: {} → {}",
        dll_src.display(),
        dll_dst.display()
    );

    Ok(())
}

#[cfg(target_os = "macos")]
fn codesign_bundle(root: &Path) -> Result<()> {
    let bundle_path = root.join("target/bundled/splug.vst3");

    let status = Command::new("codesign")
        .arg("-s")
        .arg("-")
        .arg("--force")
        .arg(&bundle_path)
        .status()
        .context("Failed to run codesign")?;

    if !status.success() {
        bail!("Codesigning failed");
    }

    println!("  ✓ Bundle signed (ad-hoc)");
    Ok(())
}

#[cfg(target_os = "macos")]
fn generate_info_plist() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>splug</string>
    <key>CFBundleIdentifier</key>
    <string>com.splug.vst3</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>splug</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string></string>
</dict>
</plist>
"#.to_string()
}

fn print_bundle_location(root: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        println!(
            "Bundle location: {}",
            root.join("target/bundled/splug.vst3").display()
        );
    }

    #[cfg(target_os = "windows")]
    {
        println!(
            "Bundle location: {}",
            root.join("target/bundled/splug.vst3").display()
        );
    }

    Ok(())
}

fn project_root() -> Result<PathBuf> {
    let output = Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .context("Failed to locate workspace root")?;

    let path = String::from_utf8(output.stdout)
        .context("Invalid UTF-8 in cargo output")?
        .trim()
        .to_string();

    let cargo_toml = PathBuf::from(path);
    cargo_toml
        .parent()
        .map(|p| p.to_path_buf())
        .context("Failed to get parent directory")
}
