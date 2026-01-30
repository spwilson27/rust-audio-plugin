use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use libloading::{Library, Symbol};
use std::ffi::{c_void, CStr};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "simple-host")]
#[command(about = "A simple VST3/CLAP plugin validator")]
struct Cli {
    /// Path to the plugin bundle/library
    #[arg(value_name = "PLUGIN_PATH")]
    path: PathBuf,

    /// Format of the plugin
    #[arg(short, long, value_enum)]
    format: Format,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Format {
    Vst3,
    Clap,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("Validating plugin at: {}", cli.path.display());
    println!("Format: {:?}", cli.format);

    if !cli.path.exists() {
        bail!("Plugin path does not exist: {}", cli.path.display());
    }

    unsafe {
        match cli.format {
            Format::Vst3 => validate_vst3(&cli.path),
            Format::Clap => validate_clap(&cli.path),
        }
    }
}

unsafe fn validate_vst3(path: &PathBuf) -> Result<()> {
    // VST3 is a bundle on macOS, so we need to point to the binary inside
    let lib_path =
        if cfg!(target_os = "macos") && path.extension().and_then(|s| s.to_str()) == Some("vst3") {
            let name = path.file_stem().unwrap().to_str().unwrap();
            // Remove .vst3 extension from stem if present (though file_stem does that)
            // standard vst3 bundle structure: Bundle.vst3/Contents/MacOS/Bundle
            path.join("Contents/MacOS").join(name)
        } else {
            path.clone()
        };

    println!("Loading library: {}", lib_path.display());
    let lib = Library::new(&lib_path).context("Failed to load VST3 library")?;

    // VST3 Entry Point: GetPluginFactory
    // signature: extern "C" fn GetPluginFactory() -> *mut IPluginFactory
    let factory_proc: Symbol<unsafe extern "C" fn() -> *mut c_void> = lib
        .get(b"GetPluginFactory")
        .context("Failed to find GetPluginFactory symbol")?;

    let factory_ptr = factory_proc();
    if factory_ptr.is_null() {
        bail!("GetPluginFactory returned null");
    }

    println!("  ✓ Found GetPluginFactory");
    println!(
        "  ✓ GetPluginFactory() returned valid pointer: {:p}",
        factory_ptr
    );

    // Note: To go deeper (calling GetFactoryInfo), we'd need VST3 FFI definitions.
    // For this simple validator, checking the symbol exists and returns non-null is a good start.

    Ok(())
}

unsafe fn validate_clap(path: &PathBuf) -> Result<()> {
    // CLAP is also a bundle on macOS, verify structure
    let lib_path =
        if cfg!(target_os = "macos") && path.extension().and_then(|s| s.to_str()) == Some("clap") {
            let name = path.file_stem().unwrap().to_str().unwrap();
            path.join("Contents/MacOS").join(name)
        } else {
            path.clone()
        };

    println!("Loading library: {}", lib_path.display());
    let lib = Library::new(&lib_path).context("Failed to load CLAP library")?;

    // CLAP Entry Point: clap_plugin_entry
    // signature: extern "C" clap_plugin_entry_t (const char *clap_version) -> *const clap_plugin_factory_t
    // or just checking for the struct symbol 'clap_entry' (version 0.1-ish)
    // Modern CLAP (>=0.24) requires 'clap_entry' symbol of type clap_plugin_entry_t

    // struct clap_plugin_entry {
    //    const char *clap_version;
    //    void (*init)(const char *plugin_path);
    //    void (*deinit)(void);
    //    const void *(*get_factory)(const char *factory_id);
    // }

    // We'll define a minimal repr to read the version
    #[repr(C)]
    struct ClapEntry {
        clap_version: *const i8,
        init: extern "C" fn(*const i8),
        deinit: extern "C" fn(),
        get_factory: extern "C" fn(*const i8) -> *const c_void,
    }

    // clap_entry is a function that returns *const ClapEntry
    // const clap_plugin_entry_t *clap_entry(const char *clap_version);
    type ClapEntryFn = unsafe extern "C" fn(*const i8) -> *const ClapEntry;

    let entry_fn: Symbol<ClapEntryFn> = lib
        .get(b"clap_entry")
        .context("Failed to find clap_entry symbol")?;

    let entry = entry_fn(std::ptr::null());
    if entry.is_null() {
        bail!("clap_entry returned null");
    }

    let entry_ref = &*entry;
    let version = CStr::from_ptr(entry_ref.clap_version).to_string_lossy();

    println!("  ✓ Found clap_entry");
    println!("  ✓ CLAP Version: {}", version);

    // Try init
    // On macOS, plugin_path should be the bundle path
    let plugin_path_str = path.to_str().unwrap();
    let c_path = std::ffi::CString::new(plugin_path_str)?;

    (entry_ref.init)(c_path.as_ptr());
    println!("  ✓ init() called successfully");

    // Try getting factory
    let factory_id = std::ffi::CString::new("clap.plugin-factory")?;
    let factory_ptr = (entry_ref.get_factory)(factory_id.as_ptr());

    if !factory_ptr.is_null() {
        println!("  ✓ Found 'clap.plugin-factory'");
    } else {
        println!("  ! 'clap.plugin-factory' not found (warning, might check others)");
    }

    (entry_ref.deinit)();
    println!("  ✓ deinit() called");

    Ok(())
}
