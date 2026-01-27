//! Vulkan swapchain management

use anyhow::{Context, Result};
use ash::vk;

pub struct Swapchain {
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    #[allow(dead_code)] // Will be used for resize handling
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    #[allow(dead_code)] // Will be used for resize handling
    format: vk::Format,
    #[allow(dead_code)] // Will be used for resize handling
    extent: vk::Extent2D,
}

impl Swapchain {
    pub fn new(context: &super::VulkanContext, width: u32, height: u32) -> Result<Self> {
        let swapchain_loader =
            ash::khr::swapchain::Device::new(context.instance(), context.device());

        // Query surface capabilities
        let surface_caps = unsafe {
            context
                .surface_loader()
                .get_physical_device_surface_capabilities(
                    context.physical_device(),
                    context.surface(),
                )?
        };

        // Query surface formats
        let surface_formats = unsafe {
            context
                .surface_loader()
                .get_physical_device_surface_formats(context.physical_device(), context.surface())?
        };

        // Choose format
        let format = surface_formats
            .iter()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(&surface_formats[0]);

        // Choose extent
        let extent = if surface_caps.current_extent.width != u32::MAX {
            surface_caps.current_extent
        } else {
            vk::Extent2D {
                width: width.clamp(
                    surface_caps.min_image_extent.width,
                    surface_caps.max_image_extent.width,
                ),
                height: height.clamp(
                    surface_caps.min_image_extent.height,
                    surface_caps.max_image_extent.height,
                ),
            }
        };

        // Choose image count (double/triple buffering)
        let image_count =
            (surface_caps.min_image_count + 1).min(if surface_caps.max_image_count > 0 {
                surface_caps.max_image_count
            } else {
                u32::MAX
            });

        // Choose composite alpha mode
        // macOS: prefer post-multiplied for transparency
        // Windows: use opaque for performance
        let composite_alpha = if surface_caps
            .supported_composite_alpha
            .contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED)
        {
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
        } else {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        };

        // Query present modes
        let present_modes = unsafe {
            context
                .surface_loader()
                .get_physical_device_surface_present_modes(
                    context.physical_device(),
                    context.surface(),
                )?
        };

        // Prefer FIFO (vsync)
        let present_mode = if present_modes.contains(&vk::PresentModeKHR::FIFO) {
            vk::PresentModeKHR::FIFO
        } else {
            present_modes[0]
        };

        // Create swapchain
        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(context.surface())
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            // Added TRANSFER_SRC for screenshot capture
            .image_usage(
                vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC,
            )
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(composite_alpha)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };

        // Get swapchain images
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        // Create image views
        let image_views: Result<Vec<_>> = images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(vk::ComponentMapping::default())
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                unsafe {
                    context
                        .device()
                        .create_image_view(&create_info, None)
                        .context("Failed to create image view")
                }
            })
            .collect();

        let image_views = image_views?;

        Ok(Self {
            swapchain_loader,
            swapchain,
            images,
            image_views,
            format: format.format,
            extent,
        })
    }

    /// Get the swapchain extent (for future resize handling)
    #[allow(dead_code)]
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    /// Get the swapchain format (for future resize handling)
    #[allow(dead_code)]
    pub fn format(&self) -> vk::Format {
        self.format
    }

    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }

    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    pub fn swapchain(&self) -> vk::SwapchainKHR {
        self.swapchain
    }

    pub fn loader(&self) -> &ash::khr::swapchain::Device {
        &self.swapchain_loader
    }

    pub fn acquire_next_image(
        &self,
        timeout: u64,
        semaphore: vk::Semaphore,
        fence: vk::Fence,
    ) -> Result<(u32, bool)> {
        unsafe {
            self.swapchain_loader
                .acquire_next_image(self.swapchain, timeout, semaphore, fence)
                .map_err(|e| anyhow::anyhow!("Failed to acquire next image: {:?}", e))
        }
    }

    pub fn cleanup(&mut self, device: &ash::Device) {
        unsafe {
            for &view in &self.image_views {
                device.destroy_image_view(view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        // Note: This is intentionally empty
        // The cleanup() method must be called manually with device reference
        // before dropping to ensure proper cleanup order
    }
}
