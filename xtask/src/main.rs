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
        /// Format to bundle (all, vst3, clap)
        #[arg(long, default_value = "all")]
        format: String,
    },
    /// Run clippy and fail on warnings
    Lint,
    /// Run coverage analysis
    Coverage {
        /// Verify coverage meets threshold (80%)
        #[arg(long)]
        verify: bool,
    },
    /// Build the project
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
        /// Build Docker image for E2E tests
        #[arg(long)]
        docker: bool,
    },
    /// Run tests
    Test {
        /// Run tests using native toolchain (no virtualization)
        #[arg(long)]
        native: bool,
        /// Run tests inside Docker container (Default on Linux)
        #[arg(long)]
        docker: bool,
        /// Run tests inside Tart VM (Default on macOS)
        #[arg(long)]
        vm: bool,
        /// Name of the Tart VM to use
        #[arg(long, default_value = "vst-test-vm")]
        vm_name: String,
        /// Optional package to test
        #[arg(short, long)]
        package: Option<String>,
    },
    /// Run all tests
    TestAll {},
    /// Validate the built plugins using simple-host
    Validate {
        /// Format to validate (vst3, clap)
        #[arg(long, default_value = "all")]
        format: String,
        /// Build in release mode
        #[arg(long, default_value_t = true)]
        release: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Bundle { release, format } => bundle(release, format),
        Commands::Lint => lint(),
        Commands::Coverage { verify } => coverage(verify),
        Commands::Build { release, docker } => build(release, docker),
        Commands::Test {
            native,
            docker,
            vm,
            vm_name,
            package,
        } => test(native, docker, vm, vm_name, package),
        Commands::TestAll {} => test_all(),
        Commands::Validate { format, release } => validate(format, release),
    }
}

fn build(release: bool, docker: bool) -> Result<()> {
    if docker {
        println!("Building Docker image...");
        let status = Command::new("docker")
            .args(["build", "-t", "rust-vst-test", "."])
            .status()
            .context("Failed to run docker build")?;

        if !status.success() {
            bail!("Docker build failed");
        }
        println!("  ✓ Docker image built: rust-vst-test");
    } else {
        println!("Building workspace...");
        let mut cmd = Command::new("cargo");
        cmd.arg("build").arg("--workspace");
        if release {
            cmd.arg("--release");
        }
        let status = cmd.status().context("Failed to run cargo build")?;
        if !status.success() {
            bail!("Build failed");
        }
        println!("  ✓ Build complete");
    }
    Ok(())
}

fn test_all() -> Result<()> {
    test(
        /*native=*/ false,
        /*docker=*/ false,
        /*vm=*/ true,
        "vst-test-vm".to_string(),
        None,
    )?;
    test(
        /*native=*/ false,
        /*docker=*/ true,
        /*vm=*/ false,
        "vst-test-vm".to_string(),
        None,
    )?;
    coverage(/*verify=*/ false)?;
    lint()?;
    validate("all".to_string(), true)?;
    Ok(())
}

fn test(
    native: bool,
    docker: bool,
    vm: bool,
    vm_name: String,
    package: Option<String>,
) -> Result<()> {
    // Determine execution mode
    let mode = if native {
        TestMode::Native
    } else if docker {
        TestMode::Docker
    } else if vm {
        TestMode::Tart
    } else if cfg!(target_os = "linux") {
        TestMode::Docker
    } else if cfg!(target_os = "macos") {
        TestMode::Tart
    } else {
        TestMode::Native
    };

    match mode {
        TestMode::Native => {
            println!("Running tests (Native)...");
            let mut cmd = Command::new("cargo");
            cmd.arg("nextest").arg("run");

            if let Some(pkg) = package {
                cmd.arg("-p").arg(pkg);
            } else {
                cmd.arg("--workspace");
            }

            let status = cmd.status().context("Failed to run cargo nextest run")?;
            if !status.success() {
                bail!("Tests failed");
            }
            println!("  ✓ Tests passed");
        }
        TestMode::Docker => {
            println!("Running tests in Docker container...");

            let pwd = std::env::current_dir()?;
            let pwd_str = pwd.to_str().context("Invalid path")?;

            // Construct the cargo nextest command to run inside docker
            let mut test_cmd = String::from("cargo nextest run --no-fail-fast");
            if let Some(pkg) = package {
                test_cmd.push_str(&format!(" -p {}", pkg));
            } else {
                test_cmd.push_str(" --workspace");
            }

            // Wrap in Xvfb and shell
            let bash_cmd = format!(
                "Xvfb :99 -screen 0 1024x768x24 & sleep 5 && DISPLAY=:99 vulkaninfo --summary && {}",
                test_cmd
            );

            // Create a temporary directory on host for golden updates
            let temp_dir =
                std::env::temp_dir().join(format!("splug_goldens_{}", std::process::id()));
            std::fs::create_dir_all(&temp_dir)
                .context("Failed to create golden update temp dir")?;
            let temp_dir_str = temp_dir.to_str().context("Invalid temp path")?;

            let status = Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "-v",
                    &format!("{}:/app", pwd_str),
                    "-v",
                    &format!("{}:/app/target/golden_updates", temp_dir_str),
                    "-e",
                    &format!("SPLUG_GOLDEN_HOST_PATH={}", temp_dir_str),
                    "-w",
                    "/app",
                    "rust-vst-test",
                    "bash",
                    "-c",
                    &bash_cmd,
                ])
                .status()
                .context("Failed to run docker container")?;

            if !status.success() {
                bail!("Tests failed in Docker");
            }
            println!("  ✓ Tests passed (Docker)");
        }
        TestMode::Tart => {
            println!("Running tests in Tart VM ('{}')...", vm_name);

            // 1. Check/Start VM
            let list_output = Command::new("tart")
                .arg("list")
                .output()
                .context("Failed to run tart list")?;

            let list_stdout = String::from_utf8_lossy(&list_output.stdout);
            let already_running = list_stdout
                .lines()
                .any(|line| line.contains(&vm_name) && line.contains("running"));

            if !already_running {
                println!("  Starting VM...");
                Command::new("tart")
                    .args(["run", "--no-graphics", &vm_name])
                    .spawn()
                    .context("Failed to start VM")?;

                println!("  Waiting for VM to boot...");
                let _ = get_vm_ip(&vm_name)?;
                std::thread::sleep(std::time::Duration::from_secs(5));
            } else {
                println!("  ✓ VM is already running");
            }

            // 2. Get IP
            let ip = get_vm_ip(&vm_name)?;
            println!("  ✓ VM IP: {}", ip);

            // 3. Wait for SSH
            wait_for_ssh(&ip)?;

            // 4. Sync Source to VM
            println!("  Syncing source code to VM (rsync)...");

            // Remove symlink if it exists (legacy), but preserve dir for incremental builds if possible
            // Note: If ~/project is a symlink, `test -L` returns true.
            let _ = Command::new("sshpass")
                .args([
                    "-p",
                    "admin",
                    "ssh",
                    "-o",
                    "StrictHostKeyChecking=no",
                    &format!("admin@{}", ip),
                    "if [ -L ~/project ]; then rm ~/project; fi && mkdir -p ~/project",
                ])
                .output();

            let rsync_args_base = [
                "-p",
                "admin",
                "rsync",
                "-avz",
                "--no-times",
                "--delete", // Delete files in VM that are removed in host (optional, but good for cleanliness)
                "--exclude",
                ".git",
                "--exclude",
                "target",
                "--exclude",
                "extern/*/target",
                "--exclude",
                "extern/*/.git",
                "--exclude",
                "node_modules",
                "-e",
                "ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null",
            ];

            let mut cmd = Command::new("sshpass");
            cmd.args(&rsync_args_base)
                .arg(".")
                .arg(&format!("admin@{}:~/project/", ip));

            let status = cmd.status().context("Failed to run rsync source")?;
            if !status.success() {
                bail!("Rsync source failed");
            }

            println!("  ✓ Sync complete");

            // 4. Run Tests in VM
            println!("  Building and Running tests in VM...");

            // Helper function to install nextest if missing
            // We use the binary installation script for speed, falling back if needed
            let mut remote_cmd = String::from(
                "export PATH=$HOME/bin:/opt/homebrew/bin:/usr/local/bin:$PATH && \
                 if ! command -v cargo-nextest &> /dev/null; then \
                    echo 'Installing cargo-nextest...'; \
                    curl -LsSf https://get.nexte.st/latest/mac | tar zxf - -C $HOME/bin || cargo install cargo-nextest --locked; \
                 fi && \
                 export SPLUG_WORKSPACE_ROOT=$HOME/project && \
                 cd ~/project && \
                 cargo nextest run"
            );

            if let Some(pkg) = package {
                remote_cmd.push_str(&format!(" -p {}", pkg));
            } else {
                remote_cmd.push_str(" --workspace");
            }

            // We stream output directly
            let status = Command::new("sshpass")
                .args([
                    "-p",
                    "admin",
                    "ssh",
                    "-t", // Force TTY for color/progress
                    "-o",
                    "StrictHostKeyChecking=no",
                    "-o",
                    "UserKnownHostsFile=/dev/null",
                    &format!("admin@{}", ip),
                    &remote_cmd,
                ])
                .status()
                .context("Failed to run SSH command")?;

            if !status.success() {
                bail!("Tests failed in VM");
            }
            println!("  ✓ All tests passed (Tart)");
        }
    }
    Ok(())
}

enum TestMode {
    Native,
    Docker,
    Tart,
}

fn get_vm_ip(name: &str) -> Result<String> {
    for _ in 0..30 {
        let output = Command::new("tart").args(["ip", name]).output()?;

        if output.status.success() {
            let ip = String::from_utf8(output.stdout)?.trim().to_string();
            if !ip.is_empty() {
                return Ok(ip);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    bail!("Timed out waiting for VM IP")
}

/// Main bundle command - orchestrates the entire build process
fn bundle(release: bool, format: String) -> Result<()> {
    println!("Building splug plugin bundle for format: {}", format);

    let root = project_root()?;
    let profile = if release { "release" } else { "debug" };

    // Step 1: Compile shaders (common for all formats)
    println!("\nStep 1/4: Compiling shaders...");
    compile_shaders(&root)?;

    let formats = if format == "all" {
        vec!["vst3", "clap"]
    } else {
        vec![format.as_str()]
    };

    for fmt in formats {
        println!("\n--- Processing format: {} ---", fmt);

        // Step 2: Build Rust library
        println!("Step 2/4: Building Rust library ({}, {})...", profile, fmt);
        build_rust_library(release, fmt)?;

        // Step 3: Create platform bundle
        println!("Step 3/4: Creating platform bundle...");
        create_bundle(&root, profile, fmt)?;

        // Step 4: Codesign (macOS only)
        #[cfg(target_os = "macos")]
        {
            println!("Step 4/4: Codesigning bundle...");
            codesign_bundle(&root, fmt)?;
        }
    }

    println!("\nBundle complete!");
    print_bundle_location(&root)?;

    Ok(())
}

fn lint() -> Result<()> {
    println!("Running clippy...");
    let status = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .status()
        .context("Failed to run cargo clippy")?;

    if !status.success() {
        bail!("Clippy checks failed!");
    }
    println!("  ✓ Clippy checks passed");
    Ok(())
}

fn coverage(verify: bool) -> Result<()> {
    println!("Running coverage analysis...");

    // Check if cargo-llvm-cov is installed
    let version_check = Command::new("cargo")
        .args(["llvm-cov", "--version"])
        .output();

    if version_check.is_err() {
        println!("  cargo-llvm-cov not found. Installing...");
        let install_status = Command::new("cargo")
            .args(["install", "cargo-llvm-cov"])
            .status()
            .context("Failed to install cargo-llvm-cov")?;

        if !install_status.success() {
            bail!("Failed to install cargo-llvm-cov");
        }
    }

    let mut cmd = Command::new("cargo");
    cmd.args(["llvm-cov", "--workspace", "--exclude", "xtask"]);

    if verify {
        cmd.args(["--fail-under-lines", "80"]);
    } else {
        cmd.arg("--summary-only");
    }

    let status = cmd.status().context("Failed to run coverage")?;

    if !status.success() {
        bail!("Coverage check failed!");
    }

    println!("  ✓ Coverage check passed");
    Ok(())
}

fn validate(format: String, release: bool) -> Result<()> {
    println!(
        "Validating plugins (format: {}, release: {})...",
        format, release
    );

    // 1. Build simple-host
    println!("  Building simple-host...");
    let status = Command::new("cargo")
        .args(["build", "-p", "simple-host"])
        .status()
        .context("Failed to build simple-host")?;

    if !status.success() {
        bail!("Failed to build simple-host");
    }

    // 2. Determine paths
    let root = project_root()?;
    let formats = if format == "all" {
        vec!["vst3", "clap"]
    } else {
        vec![format.as_str()]
    };

    let host_bin = root.join("target/debug/simple-host"); // simple-host is always debug for now? or should follow release flag?
                                                          // Let's just use debug for validator for speed, key is the plugin being tested.

    for fmt in formats {
        let bundle_ext = if fmt == "vst3" { "vst3" } else { "clap" };
        let bundle_path = root.join(format!("target/bundled/splug.{}", bundle_ext));

        if !bundle_path.exists() {
            println!("  x Bundle not found: {}", bundle_path.display());
            println!("    (Run 'cargo xtask bundle' first)");
            continue;
        }

        println!("  Validating {}...", bundle_path.display());
        let status = Command::new(&host_bin)
            .arg(&bundle_path)
            .arg("--format")
            .arg(fmt)
            .status()
            .context(format!("Failed to run simple-host for {}", fmt))?;

        if !status.success() {
            bail!("Validation failed for {}", fmt);
        }
        println!("  ✓ Validation passed for {}", fmt);
    }

    Ok(())
}

/// Compile GLSL shaders to SPIR-V
fn compile_shaders(root: &Path) -> Result<()> {
    let shader_dir = root.join("plugin/src/shaders");
    let output_dir = root.join("target/shaders");

    std::fs::create_dir_all(&output_dir).context("Failed to create shader output directory")?;

    if !shader_dir.exists() {
        println!("  No shader directory found, skipping shader compilation");
        return Ok(());
    }

    // Find glslc in PATH or common Vulkan SDK locations
    let glslc_path = find_glslc();

    if glslc_path.is_none() {
        println!("  glslc not found - skipping shader compilation");
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
        println!("  No shaders found to compile");
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
/// Build the Rust library
fn build_rust_library(release: bool, format: &str) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--package").arg("splug").arg("--lib");

    if release {
        cmd.arg("--release");
    }

    // Enable the specific feature for this format
    cmd.arg(format!("--features={}", format));

    let status = cmd.status().context("Failed to run cargo build")?;

    if !status.success() {
        bail!("Cargo build failed");
    }

    // Rename the output to include format
    let root = project_root()?;
    let profile_dir = if release { "release" } else { "debug" };

    // Source is always libsplug.dylib/dll/so
    #[cfg(target_os = "macos")]
    let lib_name = "libsplug.dylib";
    #[cfg(target_os = "windows")]
    let lib_name = "splug.dll";
    #[cfg(target_os = "linux")]
    let lib_name = "libsplug.so";

    let src = root.join(format!("target/{}/{}", profile_dir, lib_name));

    // Dest is libsplug.<format>.dylib or splug.<format>.dll
    #[cfg(target_os = "macos")]
    let dst_name = format!("libsplug.{}.dylib", format);
    #[cfg(target_os = "windows")]
    let dst_name = format!("splug.{}.dll", format);
    #[cfg(target_os = "linux")]
    let dst_name = format!("libsplug.{}.so", format);

    let dst = root.join(format!("target/{}/{}", profile_dir, dst_name));

    if !src.exists() {
        bail!("Build artifact not found at: {}", src.display());
    }

    std::fs::copy(&src, &dst).context("Failed to rename build artifact")?;
    println!("  ✓ Renamed artifact to: {}", dst_name);

    println!("  ✓ Rust library compiled");
    Ok(())
}

/// Create platform-specific bundle structure
#[allow(unused_variables)]
fn create_bundle(root: &Path, profile: &str, format: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        create_macos_bundle(root, profile, format)
    }

    #[cfg(target_os = "windows")]
    {
        create_windows_bundle(root, profile, format)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        bail!("Unsupported platform for bundle creation")
    }
}

#[cfg(target_os = "macos")]
fn create_macos_bundle(root: &Path, profile: &str, format: &str) -> Result<()> {
    let bundle_ext = if format == "vst3" { "vst3" } else { "clap" };
    let bundle_dir = root.join(format!("target/bundled/splug.{}", bundle_ext));
    let contents_dir = bundle_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");

    // Create bundle directory structure
    std::fs::create_dir_all(&macos_dir).context("Failed to create bundle directories")?;

    // Copy dylib
    let dylib_src = root.join(format!("target/{}/libsplug.{}.dylib", profile, format));
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
    let plist_content = generate_info_plist(format);
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
fn create_windows_bundle(root: &Path, profile: &str, format: &str) -> Result<()> {
    let bundle_dir = root.join("target/bundled");
    std::fs::create_dir_all(&bundle_dir).context("Failed to create bundle directory")?;

    let dll_src = root.join(format!("target/{}/splug.{}.dll", profile, format));
    let ext = if format == "vst3" { "vst3" } else { "clap" };
    let dll_dst = bundle_dir.join(format!("splug.{}", ext));

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
fn codesign_bundle(root: &Path, format: &str) -> Result<()> {
    let bundle_ext = if format == "vst3" { "vst3" } else { "clap" };
    let bundle_path = root.join(format!("target/bundled/splug.{}", bundle_ext));

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
fn generate_info_plist(format: &str) -> String {
    let ident = if format == "vst3" {
        "com.splug.vst3"
    } else {
        "com.splug.clap"
    };

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>splug</string>
    <key>CFBundleIdentifier</key>
    <string>{}</string>
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
"#,
        ident
    )
}

#[allow(unused_variables)]
fn print_bundle_location(root: &Path) -> Result<()> {
    let formats = ["vst3", "clap"];

    for fmt in formats {
        #[cfg(target_os = "macos")]
        {
            let ext = if fmt == "vst3" { "vst3" } else { "clap" };
            let path = root.join(format!("target/bundled/splug.{}", ext));
            if path.exists() {
                println!("Bundle location ({}): {}", fmt, path.display());
            }
        }

        #[cfg(target_os = "windows")]
        {
            let ext = if fmt == "vst3" { "vst3" } else { "clap" };
            let path = root.join(format!("target/bundled/splug.{}", ext));
            if path.exists() {
                println!("Bundle location ({}): {}", fmt, path.display());
            }
        }
    }

    Ok(())
}

fn wait_for_ssh(ip: &str) -> Result<()> {
    println!("  Waiting for SSH availability...");
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < 15 {
        let status = Command::new("sshpass")
            .args([
                "-p",
                "admin",
                "ssh",
                "-o",
                "ConnectTimeout=1",
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "UserKnownHostsFile=/dev/null",
                &format!("admin@{}", ip),
                "exit 0",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if let Ok(status) = status {
            if status.success() {
                println!("  ✓ SSH operational");
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    bail!("VM unresponsive to SSH (timed out after 15s)")
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
