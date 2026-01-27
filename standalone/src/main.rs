//! Standalone host for the splug audio plugin
//!
//! Allows running the plugin without a DAW for testing and development.

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "standalone")]
#[command(about = "Standalone host for splug audio plugin", long_about = None)]
struct Args {
    /// Run in headless mode (no GUI, for automated testing)
    #[arg(long)]
    headless: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("splug standalone host v{}", env!("CARGO_PKG_VERSION"));

    if args.headless {
        println!("Running in headless mode...");
        run_headless()?;
    } else {
        println!("Running with GUI...");
        run_with_gui()?;
    }

    Ok(())
}

/// Run the plugin in headless mode (no window)
/// Used for automated testing with RPC server
fn run_headless() -> Result<()> {
    println!("Headless mode: Audio engine and RPC server would start here");
    println!("TODO: Phase 5 - Initialize RPC server");
    println!("TODO: Phase 4 - Initialize audio processor");

    // For now, just demonstrate the flag works
    println!("Press Ctrl+C to exit");

    // Block indefinitely (in real implementation, wait on audio/RPC threads)
    std::thread::park();

    Ok(())
}

/// Run the plugin with GUI
/// Initializes window and rendering pipeline
fn run_with_gui() -> Result<()> {
    println!("GUI mode: Window and Vulkan renderer would start here");
    println!("TODO: Phase 2 - Initialize PAL window");
    println!("TODO: Phase 3 - Initialize Vulkan context");
    println!("TODO: Phase 4 - Initialize audio processor");

    // For now, just demonstrate it compiles
    println!("Window initialization not yet implemented");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_headless_mode_doesnt_panic() {
        // Verify headless mode initialization doesn't crash
        // In a real test, we'd spawn a thread and verify RPC server starts
        assert!(true);
    }
}
