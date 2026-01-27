//! Basic renderer for clear-color rendering

use anyhow::{Context, Result};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::time::Instant;

use super::{Swapchain, VulkanContext};

pub struct Renderer {
    context: VulkanContext,
    swapchain: Swapchain,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,
    max_frames_in_flight: usize,
    clear_color: [f32; 4],
    // Frame timing
    frame_times: Vec<f32>, // Last N frame times in milliseconds
    last_frame_time: Option<Instant>,
    current_fps: f32,
}

impl Renderer {
    pub fn new(
        window_handle: &(impl HasWindowHandle + HasDisplayHandle),
        width: u32,
        height: u32,
    ) -> Result<Self> {
        // Create Vulkan context
        let context = VulkanContext::new(window_handle)?;

        // Create swapchain
        let swapchain = Swapchain::new(&context, width, height)?;

        // Create command pool
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(context.graphics_queue_family())
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe {
            context
                .device()
                .create_command_pool(&pool_info, None)
                .context("Failed to create command pool")?
        };

        // Create command buffers
        let max_frames_in_flight = swapchain.image_views().len().min(2);
        let alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(max_frames_in_flight as u32);

        let command_buffers = unsafe {
            context
                .device()
                .allocate_command_buffers(&alloc_info)
                .context("Failed to allocate command buffers")?
        };

        // Create synchronization objects
        // Use per-swapchain-image semaphores to avoid reuse issues
        let num_images = swapchain.image_views().len();
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available_semaphores = Vec::new();
        let mut render_finished_semaphores = Vec::new();
        let mut in_flight_fences = Vec::new();

        // Create semaphores for each swapchain image
        for _ in 0..num_images {
            unsafe {
                image_available_semaphores
                    .push(context.device().create_semaphore(&semaphore_info, None)?);
                render_finished_semaphores
                    .push(context.device().create_semaphore(&semaphore_info, None)?);
            }
        }

        // Create fences for frames in flight (can be fewer than images)
        let max_frames_in_flight = num_images.min(2);
        for _ in 0..max_frames_in_flight {
            unsafe {
                in_flight_fences.push(context.device().create_fence(&fence_info, None)?);
            }
        }

        Ok(Self {
            context,
            swapchain,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            current_frame: 0,
            max_frames_in_flight,
            clear_color: [0.0, 0.0, 0.2, 1.0], // Dark blue
            frame_times: Vec::with_capacity(60),
            last_frame_time: None,
            current_fps: 0.0,
        })
    }

    /// Set the clear color
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
    }

    /// Draw a single frame
    pub fn draw_frame(&mut self) -> Result<()> {
        // Update frame timing
        self.update_frame_timing();

        let device = self.context.device();

        // Wait for previous frame
        unsafe {
            device.wait_for_fences(&[self.in_flight_fences[self.current_frame]], true, u64::MAX)?;
            device.reset_fences(&[self.in_flight_fences[self.current_frame]])?;
        }

        // Acquire next image - must acquire before we know the index!
        // Use current_frame to cycle through available sempaphores for acquire
        let acquire_semaphore_index = self.current_frame % self.image_available_semaphores.len();
        let (image_index, _is_suboptimal) = self.swapchain.acquire_next_image(
            u64::MAX,
            self.image_available_semaphores[acquire_semaphore_index],
            vk::Fence::null(),
        )?;

        // Record command buffer
        let command_buffer = self.command_buffers[self.current_frame];

        unsafe {
            device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default();
            device.begin_command_buffer(command_buffer, &begin_info)?;

            // Simple clear operation (no render pass for now)
            let image = self
                .swapchain
                .loader()
                .get_swapchain_images(self.swapchain.swapchain())?[image_index as usize];

            // Transition image to  transfer dst
            let barrier = vk::ImageMemoryBarrier::default()
                .image(image)
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            // Clear color
            let clear_color = vk::ClearColorValue {
                float32: self.clear_color,
            };
            let range = vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };

            device.cmd_clear_color_image(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear_color,
                &[range],
            );

            // Draw FPS overlay
            self.draw_fps_overlay(command_buffer, image);

            // Transition to present
            let barrier = vk::ImageMemoryBarrier::default()
                .image(image)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::empty())
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            device.end_command_buffer(command_buffer)?;
        }

        // Submit - use image-specific semaphores
        // Wait on the "acquire" semaphore, signal the image-specific "render finished" semaphore
        let wait_semaphores = [self.image_available_semaphores[acquire_semaphore_index]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[image_index as usize]];
        let command_buffers = [command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            device.queue_submit(
                self.context.graphics_queue(),
                &[submit_info],
                self.in_flight_fences[self.current_frame],
            )?;
        }

        // Present
        let swapchains = [self.swapchain.swapchain()];
        let image_indices = [image_index];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        unsafe {
            self.swapchain
                .loader()
                .queue_present(self.context.present_queue(), &present_info)
                .ok(); // Ignore OUT_OF_DATE for now
        }

        // Advance frame
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;

        Ok(())
    }

    /// Update frame timing and calculate FPS
    fn update_frame_timing(&mut self) {
        let now = Instant::now();

        if let Some(last) = self.last_frame_time {
            let frame_time_ms = now.duration_since(last).as_secs_f32() * 1000.0;

            // Rolling window of 60 frames
            if self.frame_times.len() >= 60 {
                self.frame_times.remove(0);
            }
            self.frame_times.push(frame_time_ms);

            // Calculate FPS from average frame time
            if !self.frame_times.is_empty() {
                let avg_frame_time: f32 =
                    self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
                self.current_fps = if avg_frame_time > 0.0 {
                    1000.0 / avg_frame_time
                } else {
                    0.0
                };
            }
        }

        self.last_frame_time = Some(now);
    }

    /// Draw FPS overlay bar (colored rectangle in bottom-left)
    fn draw_fps_overlay(&self, command_buffer: vk::CommandBuffer, image: vk::Image) {
        let fps = self.current_fps;
        let (_width, height) = (
            self.swapchain.extent().width,
            self.swapchain.extent().height,
        );

        // Bar dimensions
        let bar_height = 20;
        let max_bar_width = 100;
        let bar_x = 10;
        let bar_y = height.saturating_sub(bar_height + 10);

        // Map FPS (0-60) to bar width (0-100px)
        let bar_width = ((fps / 60.0) * max_bar_width as f32)
            .min(max_bar_width as f32)
            .max(0.0) as u32;

        // Color based on performance
        let color = if fps > 55.0 {
            [0.0, 1.0, 0.0, 1.0] // Green
        } else if fps > 45.0 {
            [1.0, 1.0, 0.0, 1.0] // Yellow
        } else {
            [1.0, 0.0, 0.0, 1.0] // Red
        };

        if bar_width == 0 {
            return; // Nothing to draw
        }

        // Clear a small rectangle for the FPS bar
        let clear_value = vk::ClearColorValue { float32: color };

        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let rect = vk::Rect2D {
            offset: vk::Offset2D {
                x: bar_x as i32,
                y: bar_y as i32,
            },
            extent: vk::Extent2D {
                width: bar_width,
                height: bar_height,
            },
        };

        unsafe {
            let device = self.context.device();

            // Use scissor and clear to draw the bar
            device.cmd_set_scissor(command_buffer, 0, &[rect]);
            device.cmd_clear_color_image(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &clear_value,
                &[subresource_range],
            );

            // Reset scissor to full screen
            let full_rect = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent(),
            };
            device.cmd_set_scissor(command_buffer, 0, &[full_rect]);
        }
    }

    /// Get current FPS
    pub fn get_fps(&self) -> f32 {
        self.current_fps
    }

    /// Get average frame time in milliseconds
    pub fn get_avg_frame_time(&self) -> f32 {
        if self.frame_times.is_empty() {
            0.0
        } else {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            let device = self.context.device();
            device.device_wait_idle().ok();

            // Destroy sync objects
            for semaphore in &self.image_available_semaphores {
                device.destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.render_finished_semaphores {
                device.destroy_semaphore(*semaphore, None);
            }
            for fence in &self.in_flight_fences {
                device.destroy_fence(*fence, None);
            }

            // Destroy command pool
            device.destroy_command_pool(self.command_pool, None);

            // Cleanup swapchain
            self.swapchain.cleanup(device);
        }
    }
}
