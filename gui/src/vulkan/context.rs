//! Vulkan rendering context management

use anyhow::{Context, Result};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, CString};

/// Vulkan context that manages instance, device, and surface
pub struct VulkanContext {
    _entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    graphics_queue_family: u32,
    present_queue: vk::Queue,
}

impl VulkanContext {
    /// Create a new Vulkan context from a window handle
    pub fn new(window_handle: &(impl HasWindowHandle + HasDisplayHandle)) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        // Create instance with required extensions
        let instance = Self::create_instance(&entry)?;

        // Setup debug messenger in debug builds
        let debug_messenger = if cfg!(debug_assertions) {
            Some(Self::setup_debug_messenger(&entry, &instance)?)
        } else {
            None
        };

        // Create surface from window handle
        let (surface, surface_loader) = Self::create_surface(&entry, &instance, window_handle)?;

        // Select physical device
        let physical_device = Self::select_physical_device(&instance, &surface, &surface_loader)?;

        // Create logical device and queues
        let (device, graphics_queue_family, graphics_queue, present_queue) =
            Self::create_device(&instance, physical_device, &surface, &surface_loader)?;

        Ok(Self {
            _entry: entry,
            instance,
            debug_messenger,
            surface,
            surface_loader,
            physical_device,
            device,
            graphics_queue,
            graphics_queue_family,
            present_queue,
        })
    }

    fn create_instance(entry: &ash::Entry) -> Result<ash::Instance> {
        let app_name = CString::new("Splug Audio Plugin").unwrap();
        let engine_name = CString::new("Splug Engine").unwrap();

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_0);

        // Required extensions
        let mut extension_names = vec![ash::khr::surface::NAME.as_ptr()];

        // Platform-specific surface extensions
        #[cfg(target_os = "macos")]
        {
            extension_names.push(ash::ext::metal_surface::NAME.as_ptr());
            extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
            // Required by portability_subset device extension
            extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
        }

        #[cfg(target_os = "windows")]
        {
            extension_names.push(ash::khr::win32_surface::NAME.as_ptr());
        }

        // Debug extensions
        if cfg!(debug_assertions) {
            extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
        }

        // Validation layers
        let layer_names = if cfg!(debug_assertions) {
            vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
        } else {
            vec![]
        };
        let layer_name_ptrs: Vec<*const i8> = layer_names.iter().map(|s| s.as_ptr()).collect();

        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_name_ptrs);

        // macOS: Enable portability enumeration
        #[cfg(target_os = "macos")]
        {
            create_info = create_info.flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);
        }

        unsafe {
            entry
                .create_instance(&create_info, None)
                .context("Failed to create Vulkan instance")
        }
    }

    fn setup_debug_messenger(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)> {
        let debug_utils = ash::ext::debug_utils::Instance::new(entry, instance);

        let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let messenger = unsafe {
            debug_utils
                .create_debug_utils_messenger(&create_info, None)
                .context("Failed to create debug messenger")?
        };

        Ok((debug_utils, messenger))
    }

    fn create_surface(
        entry: &ash::Entry,
        instance: &ash::Instance,
        window_handle: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<(vk::SurfaceKHR, ash::khr::surface::Instance)> {
        let display_handle = window_handle
            .display_handle()
            .map_err(|e| anyhow::anyhow!("Failed to get display handle: {:?}", e))?;
        let window_handle = window_handle
            .window_handle()
            .map_err(|e| anyhow::anyhow!("Failed to get window handle: {:?}", e))?;

        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                display_handle.as_raw(),
                window_handle.as_raw(),
                None,
            )
            .context("Failed to create Vulkan surface")?
        };

        let surface_loader = ash::khr::surface::Instance::new(entry, instance);

        Ok((surface, surface_loader))
    }

    fn select_physical_device(
        instance: &ash::Instance,
        surface: &vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
    ) -> Result<vk::PhysicalDevice> {
        let devices = unsafe { instance.enumerate_physical_devices()? };

        devices
            .into_iter()
            .max_by_key(|device| {
                let props = unsafe { instance.get_physical_device_properties(*device) };

                // Check if device supports required queue families
                let queue_families =
                    unsafe { instance.get_physical_device_queue_family_properties(*device) };

                let has_graphics = queue_families
                    .iter()
                    .any(|qf| qf.queue_flags.contains(vk::QueueFlags::GRAPHICS));

                let has_present = queue_families.iter().enumerate().any(|(i, _)| unsafe {
                    surface_loader
                        .get_physical_device_surface_support(*device, i as u32, *surface)
                        .unwrap_or(false)
                });

                if !has_graphics || !has_present {
                    return 0;
                }

                // Score: discrete GPU > integrated GPU
                match props.device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 100,
                    _ => 1,
                }
            })
            .context("No suitable GPU found")
    }

    fn create_device(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
        surface_loader: &ash::khr::surface::Instance,
    ) -> Result<(ash::Device, u32, vk::Queue, vk::Queue)> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        // Find graphics queue family
        let graphics_family = queue_families
            .iter()
            .enumerate()
            .find(|(_, qf)| qf.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .map(|(i, _)| i as u32)
            .context("No graphics queue family found")?;

        // Find present queue family
        let present_family = queue_families
            .iter()
            .enumerate()
            .find(|(i, _)| unsafe {
                surface_loader
                    .get_physical_device_surface_support(physical_device, *i as u32, *surface)
                    .unwrap_or(false)
            })
            .map(|(i, _)| i as u32)
            .context("No present queue family found")?;

        let queue_priorities = [1.0];
        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(graphics_family)
            .queue_priorities(&queue_priorities)];

        let device_extension_names = vec![
            ash::khr::swapchain::NAME.as_ptr(),
            #[cfg(target_os = "macos")]
            c"VK_KHR_portability_subset".as_ptr(),
        ];

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&device_extension_names);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .context("Failed to create logical device")?
        };

        let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family, 0) };

        Ok((device, graphics_family, graphics_queue, present_queue))
    }

    /// Get the physical device handle.
    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    /// Get the logical device handle
    pub fn device(&self) -> &ash::Device {
        &self.device
    }

    /// Get the graphics queue
    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    /// Get the graphics queue family index
    pub fn graphics_queue_family(&self) -> u32 {
        self.graphics_queue_family
    }

    /// Get the present queue
    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }

    /// Get the surface
    pub fn surface(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// Get the surface loader
    pub fn surface_loader(&self) -> &ash::khr::surface::Instance {
        &self.surface_loader
    }

    /// Get the instance
    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    /// Find a memory type index that matches the filter and properties.
    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<u32> {
        let mem_properties = unsafe {
            self.instance
                .get_physical_device_memory_properties(self.physical_device)
        };

        for i in 0..mem_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && (mem_properties.memory_types[i as usize].property_flags & properties)
                    == properties
            {
                return Ok(i);
            }
        }

        anyhow::bail!("Failed to find suitable memory type")
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().ok();
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);

            if let Some((debug_utils, messenger)) = self.debug_messenger.take() {
                debug_utils.destroy_debug_utils_messenger(messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

/// Vulkan debug callback
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = CStr::from_ptr((*p_callback_data).p_message);

    if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        tracing::error!("[Vulkan Error] {:?}", message);
    } else if message_severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        tracing::warn!("[Vulkan Warning] {:?}", message);
    }

    vk::FALSE
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vulkan_context_compiles() {
        // Basic compilation test
    }
}
