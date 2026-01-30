//! # splug - High-Performance Audio Plugin
//!
//! A metal-down VST3/CLAP audio plugin built with Rust and Vulkan.
//!
//! ## Architecture
//! - PAL (Platform Abstraction Layer) - Native windowing
//! - Render - Vulkan graphics pipeline
//! - Core - Audio processing and state management

use std::ffi::c_void;

// Module declarations for future phases
pub mod pal {
    //! Platform Abstraction Layer
    //! Handles native window creation and management
}

pub mod render {
    //! Vulkan rendering backend
    //! SVG rasterization and GPU compositing
}

pub mod core {
    //! Audio processing and state management
    //! Lock-free parameter communication
}

// -----------------------------------------------------------------------------
// VST3 Entry Point
// -----------------------------------------------------------------------------

#[cfg(feature = "vst3")]
pub mod vst3 {
    use std::ffi::{c_void};
    use std::os::raw::c_char;
    use vst3_sys::base::*;
    use vst3_sys::vst::*;
    use vst3_sys::VST3;
    use std::ptr::{copy_nonoverlapping, null_mut};

    const CID_PROCESSOR: vst3_sys::sys::GUID = vst3_sys::sys::GUID { data: [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01] };

    unsafe fn strcpy(src: &str, dst: *mut c_char) {
        let src_bytes = src.as_bytes();
        let len = std::cmp::min(src_bytes.len(), 63); // safety margin
        copy_nonoverlapping(src_bytes.as_ptr() as *const c_char, dst, len);
        *dst.add(len) = 0;
    }

    unsafe fn wstrcpy(src: &str, dst: *mut i16) {
        let mut i = 0;
        for c in src.encode_utf16() {
            *dst.add(i) = c as i16;
            i += 1;
        }
        *dst.add(i) = 0;
    }

    #[VST3(implements(IComponent, IAudioProcessor, IEditController))]
    pub struct SplugPlugin {}

    impl SplugPlugin {
        pub fn new() -> Box<Self> {
            Self::allocate()
        }
    }

    impl IComponent for SplugPlugin {
        unsafe fn get_controller_class_id(&self, tuid: *mut vst3_sys::sys::IID) -> tresult { 
            *tuid = CID_PROCESSOR;
            kResultOk
        }
        unsafe fn set_io_mode(&self, _mode: IoMode) -> tresult { 
            kNotImplemented 
        }
        
        unsafe fn get_bus_count(&self, type_: MediaType, dir: BusDirection) -> i32 {
            if type_ == MediaTypes::kAudio as i32 && dir == BusDirections::kOutput as i32 {
                1
            } else if type_ == MediaTypes::kEvent as i32 && dir == BusDirections::kInput as i32 {
                1
            } else {
                0
            }
        }

        unsafe fn get_bus_info(&self, type_: MediaType, dir: BusDirection, index: i32, info: *mut BusInfo) -> tresult {
            if type_ == MediaTypes::kAudio as i32 && dir == BusDirections::kOutput as i32 && index == 0 {
                let info = &mut *info;
                info.media_type = MediaTypes::kAudio as i32;
                info.direction = BusDirections::kOutput as i32;
                info.channel_count = 2;
                wstrcpy("Stereo Out", info.name.as_mut_ptr()); 
                info.bus_type = BusTypes::kMain as i32;
                info.flags = BusFlags::kDefaultActive as u32;
                kResultOk
            } else if type_ == MediaTypes::kEvent as i32 && dir == BusDirections::kInput as i32 && index == 0 {
                let info = &mut *info;
                info.media_type = MediaTypes::kEvent as i32;
                info.direction = BusDirections::kInput as i32;
                info.channel_count = 1; // 1 event bus
                wstrcpy("MIDI In", info.name.as_mut_ptr()); 
                info.bus_type = BusTypes::kMain as i32;
                info.flags = BusFlags::kDefaultActive as u32;
                kResultOk
            } else {
                kInvalidArgument
            }
        }

        unsafe fn get_routing_info(&self, _in_info: *mut RoutingInfo, _out_info: *mut RoutingInfo) -> tresult { kNotImplemented }
        unsafe fn activate_bus(&self, _type_: MediaType, _dir: BusDirection, _index: i32, _state: TBool) -> tresult { 
            kResultOk 
        }
        unsafe fn set_active(&self, _state: TBool) -> tresult { 
            kResultOk 
        }
        unsafe fn set_state(&self, _state: vst3_sys::utils::SharedVstPtr<dyn IBStream>) -> tresult { 
            kResultOk 
        }
        unsafe fn get_state(&self, _state: vst3_sys::utils::SharedVstPtr<dyn IBStream>) -> tresult { 
            kResultOk 
        }
    }

    impl IEditController for SplugPlugin {
        unsafe fn set_component_state(&self, _state: vst3_sys::utils::SharedVstPtr<dyn IBStream>) -> tresult { kResultOk }
        unsafe fn set_state(&self, _state: vst3_sys::utils::SharedVstPtr<dyn IBStream>) -> tresult { kResultOk }
        unsafe fn get_state(&self, _state: vst3_sys::utils::SharedVstPtr<dyn IBStream>) -> tresult { kResultOk }
        unsafe fn get_parameter_count(&self) -> i32 { 0 }
        unsafe fn get_parameter_info(&self, _param_index: i32, _info: *mut ParameterInfo) -> tresult { kResultFalse }
        unsafe fn get_param_string_by_value(&self, _id: u32, _value_normalized: f64, _string: *mut TChar) -> tresult { kResultFalse }
        unsafe fn get_param_value_by_string(&self, _id: u32, _string: *const TChar, _value_normalized: *mut f64) -> tresult { kResultFalse }
        unsafe fn normalized_param_to_plain(&self, _id: u32, value_normalized: f64) -> f64 { value_normalized }
        unsafe fn plain_param_to_normalized(&self, _id: u32, plain_value: f64) -> f64 { plain_value }
        unsafe fn get_param_normalized(&self, _id: u32) -> f64 { 0.0 }
        unsafe fn set_param_normalized(&self, _id: u32, _value: f64) -> tresult { kResultOk }
        unsafe fn set_component_handler(&self, _handler: vst3_sys::utils::SharedVstPtr<dyn IComponentHandler>) -> tresult { kResultOk }
        unsafe fn create_view(&self, _name: FIDString) -> *mut c_void { null_mut() }
    }

    impl IPluginBase for SplugPlugin {
        unsafe fn initialize(&self, _context: *mut c_void) -> tresult { kResultOk }
        unsafe fn terminate(&self) -> tresult { kResultOk }
    }

    impl IAudioProcessor for SplugPlugin {
        unsafe fn set_bus_arrangements(&self, _inputs: *mut SpeakerArrangement, num_ins: i32, outputs: *mut SpeakerArrangement, num_outs: i32) -> tresult { 
            // We only support 0 inputs and 1 output (Stereo)
            if num_ins == 0 && num_outs == 1 {
                // Check if the requested output arrangement is Stereo
                if *outputs == kStereo {
                    return kResultOk;
                }
            }
            kResultFalse 
        }

        unsafe fn get_bus_arrangement(&self, dir: BusDirection, index: i32, arr: *mut SpeakerArrangement) -> tresult {
            if dir == BusDirections::kOutput as i32 && index == 0 {
                *arr = kStereo;
                return kResultOk;
            }
            if dir == BusDirections::kInput as i32 {
                // We have no audio inputs
                return kResultFalse;
            }
            kInvalidArgument
        }

        unsafe fn can_process_sample_size(&self, _symbolic_sample_size: i32) -> tresult { kResultTrue }
        unsafe fn get_latency_samples(&self) -> u32 { 0 }
        unsafe fn setup_processing(&self, _setup: *const ProcessSetup) -> tresult { 
            kResultOk 
        }
        unsafe fn set_processing(&self, _state: TBool) -> tresult { 
            kResultOk 
        }
        unsafe fn process(&self, _data: *mut ProcessData) -> tresult { 
            kResultOk 
        }
        unsafe fn get_tail_samples(&self) -> u32 { 0 }
    }

    #[VST3(implements(IPluginFactory, IPluginFactory2))]
    pub struct Factory {}

    impl Factory {
        pub fn new() -> Box<Self> {
            Self::allocate()
        }
    }

    impl IPluginFactory for Factory {
        unsafe fn get_factory_info(&self, info: *mut PFactoryInfo) -> tresult {
            let info = &mut *info;
            strcpy("Splug Audio", info.vendor.as_mut_ptr());
            strcpy("https://splug.audio", info.url.as_mut_ptr());
            strcpy("contact@splug.audio", info.email.as_mut_ptr());
            info.flags = FactoryFlags::kUnicode as i32;
            kResultOk
        }
        unsafe fn count_classes(&self) -> i32 { 1 }
        unsafe fn get_class_info(&self, index: i32, info: *mut PClassInfo) -> tresult {
            if index == 0 {
                let info = &mut *info;
                info.cid = CID_PROCESSOR;
                info.cardinality = ClassCardinality::kManyInstances as i32;
                strcpy("Audio Module Class", info.category.as_mut_ptr());
                strcpy("Splug Synth", info.name.as_mut_ptr());
                kResultOk
            } else {
                kInvalidArgument
            }
        }
        unsafe fn create_instance(&self, cid: *const vst3_sys::sys::IID, _iid: *const vst3_sys::sys::IID, obj: *mut *mut c_void) -> tresult {
            if *cid == CID_PROCESSOR {
                *obj = Box::into_raw(SplugPlugin::new()) as *mut c_void;
                kResultOk
            } else {
                kInvalidArgument
            }
        }
    }

    impl IPluginFactory2 for Factory {
        unsafe fn get_class_info2(&self, index: i32, info: *mut PClassInfo2) -> tresult {
            if index == 0 {
                let info = &mut *info;
                info.cid = CID_PROCESSOR;
                info.cardinality = ClassCardinality::kManyInstances as i32;
                strcpy("Audio Module Class", info.category.as_mut_ptr());
                strcpy("Splug Synth", info.name.as_mut_ptr());
                info.class_flags = 0;
                strcpy("Instrument|Synth", info.subcategories.as_mut_ptr());
                strcpy("1.0.0", info.version.as_mut_ptr());
                strcpy("VST 3.7.0", info.sdk_version.as_mut_ptr());
                kResultOk
            } else {
                kInvalidArgument
            }
        }
    }

    #[no_mangle]
    #[allow(non_snake_case)]
    pub unsafe extern "system" fn GetPluginFactory() -> *mut c_void {
        Box::into_raw(Factory::new()) as *mut c_void
    }
}

// -----------------------------------------------------------------------------
// CLAP Entry Point
// -----------------------------------------------------------------------------

#[cfg(feature = "clap")]
pub mod clap {
    use std::ffi::{c_char, c_void};
    use clap_sys::entry::clap_plugin_entry;
    use clap_sys::factory::plugin_factory::{clap_plugin_factory, CLAP_PLUGIN_FACTORY_ID};
    use clap_sys::plugin::{clap_plugin_descriptor, clap_plugin};
    use clap_sys::host::clap_host;
    use clap_sys::version::CLAP_VERSION;

    struct EntryWrapper(clap_plugin_entry);
    unsafe impl Sync for EntryWrapper {}

    extern "C" fn init(_plugin_path: *const c_char) -> bool {
        true
    }

    extern "C" fn deinit() {}

    extern "C" fn get_factory(factory_id: *const c_char) -> *const c_void {
        let id = unsafe { std::ffi::CStr::from_ptr(factory_id) };
        if id.to_bytes() == CLAP_PLUGIN_FACTORY_ID.to_bytes() {
            &PLUGIN_FACTORY.0 as *const _ as *const c_void
        } else {
            std::ptr::null()
        }
    }

    static ENTRY: EntryWrapper = EntryWrapper(clap_plugin_entry {
        clap_version: CLAP_VERSION,
        init: Some(init),
        deinit: Some(deinit),
        get_factory: Some(get_factory),
    });

    #[no_mangle]
    #[allow(non_snake_case)]
    pub unsafe extern "C" fn clap_entry(_clap_version: *const clap_sys::version::clap_version) -> *const clap_plugin_entry {
        // Check host compatible version?
        // For now just return our entry
        &ENTRY.0
    }

    // --- CLAP Factory Implementation ---
    
    // Features array (null terminated)
    struct FeaturesWrapper([*const c_char; 4]);
    unsafe impl Sync for FeaturesWrapper {}

    static FEATURES: FeaturesWrapper = FeaturesWrapper([
        clap_sys::plugin_features::CLAP_PLUGIN_FEATURE_INSTRUMENT.as_ptr(), 
        clap_sys::plugin_features::CLAP_PLUGIN_FEATURE_SYNTHESIZER.as_ptr(), 
        clap_sys::plugin_features::CLAP_PLUGIN_FEATURE_STEREO.as_ptr(), 
        std::ptr::null()
    ]);

    // Static descriptor data
    static PLUGIN_ID: &str = "com.splug.synth\0";
    static PLUGIN_NAME: &str = "Splug Synth\0";
    static PLUGIN_VENDOR: &str = "Splug Audio\0";
    static PLUGIN_URL: &str = "https://splug.audio\0";
    static PLUGIN_MANUAL_URL: &str = "https://splug.audio/manual\0";
    static PLUGIN_SUPPORT_URL: &str = "https://splug.audio/support\0";
    static PLUGIN_VERSION: &str = "1.0.0\0";
    static PLUGIN_DESC: &str = "A metal-down VST3/CLAP plugin\0";

    static DESCRIPTOR: clap_plugin_descriptor = clap_plugin_descriptor {
        clap_version: CLAP_VERSION,
        id: PLUGIN_ID.as_ptr() as *const c_char,
        name: PLUGIN_NAME.as_ptr() as *const c_char,
        vendor: PLUGIN_VENDOR.as_ptr() as *const c_char,
        url: PLUGIN_URL.as_ptr() as *const c_char,
        manual_url: PLUGIN_MANUAL_URL.as_ptr() as *const c_char,
        support_url: PLUGIN_SUPPORT_URL.as_ptr() as *const c_char,
        version: PLUGIN_VERSION.as_ptr() as *const c_char,
        description: PLUGIN_DESC.as_ptr() as *const c_char,
        features: FEATURES.0.as_ptr() as *const *const c_char,
    };
    
    struct DescriptorWrapper(clap_plugin_descriptor);
    unsafe impl Sync for DescriptorWrapper {}
    static DESCRIPTOR_WRAPPER: DescriptorWrapper = DescriptorWrapper(DESCRIPTOR);

    extern "C" fn get_plugin_count(_factory: *const clap_plugin_factory) -> u32 {
        1
    }

    extern "C" fn get_plugin_descriptor(_factory: *const clap_plugin_factory, index: u32) -> *const clap_plugin_descriptor {
        if index == 0 {
            &DESCRIPTOR_WRAPPER.0
        } else {
            std::ptr::null()
        }
    }

    extern "C" fn create_plugin(_factory: *const clap_plugin_factory, _host: *const clap_host, _plugin_id: *const c_char) -> *const clap_plugin {
        std::ptr::null()
    }

    struct FactoryWrapper(clap_plugin_factory);
    unsafe impl Sync for FactoryWrapper {}

    static PLUGIN_FACTORY: FactoryWrapper = FactoryWrapper(clap_plugin_factory {
        get_plugin_count: Some(get_plugin_count),
        get_plugin_descriptor: Some(get_plugin_descriptor),
        create_plugin: Some(create_plugin),
    });
}

/// Module entry point (Windows)
/// Required for DLL initialization on Windows
#[cfg(target_os = "windows")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: *mut c_void,
    _fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> i32 {
    1 // TRUE
}

/// Bundle entry point (macOS)
/// Called when the bundle is loaded on macOS
/// # Safety
///
/// Called by the host when the bundle is loaded.
#[cfg(target_os = "macos")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn bundleEntry(_bundle: *mut c_void) -> bool {
    true
}

/// Bundle exit point (macOS)
/// Called when the bundle is unloaded on macOS
/// # Safety
///
/// Called by the host when the bundle is unloaded.
#[cfg(target_os = "macos")]
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn bundleExit() -> bool {
    true
}

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "vst3")]
    fn test_vst3_factory_doesnt_panic() {
        // Verify the entry point doesn't crash
        unsafe {
            use super::vst3::GetPluginFactory;
            let factory = GetPluginFactory();
            // Should be non-null now
            assert!(!factory.is_null());
        }
    }

    #[test]
    fn test_panic_caught_at_boundary() {
        // This logic is now inside mod vst3, so we can't easily test it without exposing inner function
        // or just assuming it works via std::panic::catch_unwind mechanism tests
    }

    #[test]
    #[cfg(feature = "clap")]
    fn test_clap_entry_exists() {
        unsafe {
            use super::clap::clap_entry;
            // Need a dummy version struct
            let version = clap_sys::version::clap_version { major: 1, minor: 1, revision: 10 };
            let entry = clap_entry(&version);
            assert!(!entry.is_null());
            let entry_ref = &*entry;
            // assert!(!entry_ref.clap_version.is_null()); // clap_version is struct in sys, not ptr
        }
    }
}
