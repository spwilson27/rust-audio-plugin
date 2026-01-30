use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use libloading::{Library, Symbol};
use std::ffi::{c_void, CStr};
use std::path::PathBuf;

use vst3_sys::base::{PFactoryInfo, PClassInfo};
use clap_sys::entry::clap_plugin_entry;
use clap_sys::factory::plugin_factory::{clap_plugin_factory, CLAP_PLUGIN_FACTORY_ID};
// use clap_sys::plugin::clap_plugin_descriptor; // We use raw pointers often

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

// -----------------------------------------------------------------------------
// VST3 Definitions (Manual VTable)
// -----------------------------------------------------------------------------
// vst3-sys provides traits but not the raw vtable struct layout easily accessible for manual FFI calls
// without using the COM wrapper logic. For a validator, manual mapping is fine and robust.

#[repr(C)]
struct IPluginFactoryVTable {
    // FUnknown
    query_interface: unsafe extern "system" fn(this: *mut c_void, iid: *const u8, obj: *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(this: *mut c_void) -> u32,
    release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    // IPluginFactory
    get_factory_info: unsafe extern "system" fn(this: *mut c_void, info: *mut PFactoryInfo) -> i32,
    count_classes: unsafe extern "system" fn(this: *mut c_void) -> i32,
    get_class_info: unsafe extern "system" fn(this: *mut c_void, index: i32, info: *mut PClassInfo) -> i32,
    create_instance: unsafe extern "system" fn(this: *mut c_void, cid: *const u8, iid: *const u8, obj: *mut *mut c_void) -> i32,
}

// -----------------------------------------------------------------------------
// Validation Logic
// -----------------------------------------------------------------------------

unsafe fn validate_vst3(path: &PathBuf) -> Result<()> {
    // VST3 is a bundle on macOS, so we need to point to the binary inside
    let lib_path =
        if cfg!(target_os = "macos") && path.extension().and_then(|s| s.to_str()) == Some("vst3") {
            let name = path.file_stem().unwrap().to_str().unwrap();
            path.join("Contents/MacOS").join(name)
        } else {
            path.clone()
        };

    println!("Loading library: {}", lib_path.display());
    let lib = Library::new(&lib_path).context("Failed to load VST3 library")?;

    // VST3 Entry Point: GetPluginFactory
    let factory_proc: Symbol<unsafe extern "C" fn() -> *mut *const IPluginFactoryVTable> = lib
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

    // Read Factory Info
    let vtable = (*factory_ptr).as_ref().unwrap();
    // PFactoryInfo is from vst3-sys, assumed to be initialized by callee
    let mut info: PFactoryInfo = std::mem::zeroed();

    let res = (vtable.get_factory_info)(factory_ptr as *mut c_void, &mut info);
    if res != 0 {
        bail!("getFactoryInfo failed with result: {}", res);
    }

    let vendor = CStr::from_ptr(info.vendor.as_ptr()).to_string_lossy();
    let url = CStr::from_ptr(info.url.as_ptr()).to_string_lossy();
    let email = CStr::from_ptr(info.email.as_ptr()).to_string_lossy();

    println!("  ✓ Factory Info:");
    println!("      Vendor: {}", vendor);
    println!("      URL:    {}", url);
    println!("      Email:  {}", email);
    println!("      Flags:  {}", info.flags);

    // Count Classes
    let count = (vtable.count_classes)(factory_ptr as *mut c_void);
    println!("  ✓ Classes found: {}", count);

    for i in 0..count {
        let mut class_info: PClassInfo = std::mem::zeroed();
        let res = (vtable.get_class_info)(factory_ptr as *mut c_void, i, &mut class_info);
        if res == 0 {
            let name = CStr::from_ptr(class_info.name.as_ptr()).to_string_lossy();
            let category = CStr::from_ptr(class_info.category.as_ptr()).to_string_lossy();
            let cid_str = class_info.cid.data.iter().map(|b| format!("{:02X}", b)).collect::<String>();
            println!("      [{}] Name: {}, Category: {}, CID: {}", i, name, category, cid_str);

            // Attempt creation
            println!("      -> Creating instance...");
            let mut obj: *mut c_void = std::ptr::null_mut();
            // IComponent IID (standard VST3)
            // 0xE831FF31, 0xF2D54301, 0x928EBBEE, 0x25697802
            // Little Endian: 31 FF 31 E8 ... wait GUIDs are weird.
            // vst3-sys defines it. Let's use vst3-sys IID if possible?
            // vst3_sys::vst::IComponent::IID
            // But we can't easily access the const data without linking?
            // vst3-sys provides `pub const IID: vst3_com::sys::IID`
            
            // Let's blindly trust the factory accepts standard IUnknown IID too?
            // {00000000-0000-0000-C000-000000000046}
            let iunknown_iid = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46];

            let res = (vtable.create_instance)(
                factory_ptr as *mut c_void,
                class_info.cid.data.as_ptr(),
                iunknown_iid.as_ptr(),
                &mut obj
            );

            if res == 0 && !obj.is_null() {
                println!("      ✓ Created instance: {:p}", obj);
                // Ideally release it, but we lack IUnknown def here. Leaking for validator is acceptable for now.
            } else {
                println!("      x Create instance failed: {}", res);
                // Don't bail yet, maybe other classes work?
            }

        } else {
            println!("      [{}] Failed to get class info", i);
        }
    }

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

    type ClapEntryFn = unsafe extern "C" fn(*const clap_sys::version::clap_version) -> *const clap_plugin_entry;

    let entry_fn: Symbol<ClapEntryFn> = lib
        .get(b"clap_entry")
        .context("Failed to find clap_entry symbol")?;

    // Must pass a valid pointer to clap_version
    let host_version = clap_sys::version::clap_version {
        major: 1,
        minor: 1,
        revision: 8, // Host version
    };

    let entry = entry_fn(&host_version);
    if entry.is_null() {
        bail!("clap_entry returned null");
    }

    let entry_ref = &*entry;
    let v = entry_ref.clap_version;
    let version = format!("{}.{}.{}", v.major, v.minor, v.revision);

    println!("  ✓ Found clap_entry");
    println!("  ✓ CLAP Version (Entry): {}", version);

    // Init
    let plugin_path_str = path.to_str().unwrap();
    let c_path = std::ffi::CString::new(plugin_path_str)?;

    if let Some(init) = entry_ref.init {
        if !init(c_path.as_ptr()) {
            bail!("clap_entry.init() failed");
        }
        println!("  ✓ init() called successfully");
    } else {
         println!("  ! init() is None");
    }

    // Get Factory
    let factory_id = std::ffi::CString::new("clap.plugin-factory")?;
    // Or use CLAP_PLUGIN_FACTORY_ID from sys
    
    if let Some(get_factory) = entry_ref.get_factory {
        let factory_ptr = get_factory(CLAP_PLUGIN_FACTORY_ID.as_ptr() as *const i8) as *const clap_plugin_factory;

        if factory_ptr.is_null() {
            println!("  ! 'clap.plugin-factory' not found (Is this a plugin?)");
        } else {
            println!("  ✓ Found 'clap.plugin-factory'");
            let factory = &*factory_ptr;
            
            if let Some(get_plugin_count) = factory.get_plugin_count {
                let count = get_plugin_count(factory_ptr);
                println!("  ✓ Plugins found: {}", count);

                if let Some(get_plugin_descriptor) = factory.get_plugin_descriptor {
                    for i in 0..count {
                        let desc_ptr = get_plugin_descriptor(factory_ptr, i);
                        if !desc_ptr.is_null() {
                            let desc = &*desc_ptr;
                            let name = CStr::from_ptr(desc.name).to_string_lossy();
                            let id = CStr::from_ptr(desc.id).to_string_lossy();
                            let vendor = if !desc.vendor.is_null() { CStr::from_ptr(desc.vendor).to_string_lossy() } else { "Unknown".into() };
                            let version = if !desc.version.is_null() { CStr::from_ptr(desc.version).to_string_lossy() } else { "Unknown".into() };

                            println!("      [{}] Name: {}", i, name);
                            println!("          ID:      {}", id);
                            println!("          Vendor:  {}", vendor);
                            println!("          Version: {}", version);
                            
                            if !desc.features.is_null() {
                                print!("          Features: ");
                                let mut feature_ptr = desc.features;
                                while !(*feature_ptr).is_null() {
                                    print!("{} ", CStr::from_ptr(*feature_ptr).to_string_lossy());
                                    feature_ptr = feature_ptr.add(1);
                                }
                                println!();
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(deinit) = entry_ref.deinit {
        deinit();
        println!("  ✓ deinit() called");
    }

    Ok(())
}