//! Basic renderer for clear-color rendering

use anyhow::{Context, Result};
use ash::vk;
use image::RgbaImage;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::time::Instant;

use super::shape_renderer::ShapeRenderer;
use super::{Swapchain, VulkanContext};

/// The main Vulkan renderer responsible for managing the swapchain,
/// render passes, and coordinating sub-renderers (Shape, Text).
pub struct Renderer {
    /// Manages the presentation surface and images
    swapchain: Swapchain,
    /// Pool for allocating command buffers
    command_pool: vk::CommandPool,
    /// One command buffer per swapchain image
    command_buffers: Vec<vk::CommandBuffer>,
    /// Semaphores signaling when an image is ready to be rendered to
    image_available_semaphores: Vec<vk::Semaphore>,
    /// Semaphores signaling when rendering is complete and image can be presented
    render_finished_semaphores: Vec<vk::Semaphore>,
    /// Fences to synchronize CPU and GPU frame submission
    in_flight_fences: Vec<vk::Fence>,

    current_frame: usize,
    max_frames_in_flight: usize,
    clear_color: [f32; 4],

    // Rendering resources
    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,

    /// Sub-renderer for 2D geometric shapes (SDF based)
    shape_renderer: ShapeRenderer,
    /// Sub-renderer for text using a dynamic font atlas
    text_renderer: super::text_renderer::TextRenderer,
    /// dynamic font texture atlas
    font_atlas: super::text_renderer::FontAtlas,

    // Frame timing
    frame_times: Vec<f32>, // Last N frame times in milliseconds
    last_frame_time: Option<Instant>,
    current_fps: f32,
    fixed_fps: Option<f32>,

    /// Logical size of the UI (points/logical pixels)
    logical_width: f32,
    logical_height: f32,

    // Context must be last to be dropped last
    context: VulkanContext,
}

impl Renderer {
    /// Creates a new Renderer instance, initializing Vulkan context, swapchain, and pipelines.
    ///
    /// # Arguments
    ///
    /// * `window_handle` - The raw window handle for surface creation.
    /// * `width` - Initial window width.
    /// * `height` - Initial window height.
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

        // Create Render Pass
        let render_pass = Self::create_render_pass(&context, swapchain.format())?;

        // Create Framebuffers
        let framebuffers = Self::create_framebuffers(&context, &swapchain, render_pass)?;

        // Create Shape Renderer
        let shape_renderer = ShapeRenderer::new(&context, render_pass)?;

        // Create Font Atlas and Text Renderer
        let font_atlas =
            super::text_renderer::FontAtlas::new(&context, command_pool, context.graphics_queue())?;
        let text_renderer =
            super::text_renderer::TextRenderer::new(&context, render_pass, &font_atlas)?;

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
            render_pass,
            framebuffers,
            shape_renderer,
            text_renderer,
            font_atlas,
            frame_times: Vec::with_capacity(60),
            last_frame_time: None,
            current_fps: 0.0,
            fixed_fps: None,
            logical_width: width as f32,
            logical_height: height as f32,
        })
    }

    fn create_render_pass(context: &VulkanContext, format: vk::Format) -> Result<vk::RenderPass> {
        let color_attachment = vk::AttachmentDescription::default()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(std::slice::from_ref(&color_attachment_ref));

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let render_pass_info = vk::RenderPassCreateInfo::default()
            .attachments(std::slice::from_ref(&color_attachment))
            .subpasses(std::slice::from_ref(&subpass))
            .dependencies(std::slice::from_ref(&dependency));

        unsafe {
            Ok(context
                .device()
                .create_render_pass(&render_pass_info, None)?)
        }
    }

    fn create_framebuffers(
        context: &VulkanContext,
        swapchain: &Swapchain,
        render_pass: vk::RenderPass,
    ) -> Result<Vec<vk::Framebuffer>> {
        let mut framebuffers = Vec::new();

        for view in swapchain.image_views() {
            let attachments = [*view];
            let create_info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(swapchain.extent().width)
                .height(swapchain.extent().height)
                .layers(1);

            unsafe {
                framebuffers.push(context.device().create_framebuffer(&create_info, None)?);
            }
        }
        Ok(framebuffers)
    }

    /// Set the clear color
    pub fn set_clear_color(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.clear_color = [r, g, b, a];
    }

    /// Set a fixed FPS value for testing (deterministic output)
    pub fn set_fixed_fps(&mut self, fps: Option<f32>) {
        self.fixed_fps = fps;
    }

    /// Draw a single frame.
    ///
    /// This acquires an image from the swapchain, records commands to clear it and draw contents,
    /// submits the commands to the GPU, and presents the image.
    pub fn draw_frame(
        &mut self,
        widgets: Option<&crate::widgets::container::WidgetContainer>,
    ) -> Result<()> {
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

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(command_buffer, &begin_info)?;

            // Begin Render Pass
            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: self.clear_color,
                },
            }];

            let render_pass_begin_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index as usize])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent(),
                })
                .clear_values(&clear_values);

            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );

            // Initialize renderers
            self.shape_renderer.begin();
            self.text_renderer.begin();

            let w = self.swapchain.extent().width as f32;
            let h = self.swapchain.extent().height as f32;

            // Render widgets if available, otherwise render test pattern
            if let Some(widgets_ref) = widgets {
                // 1. Render Base Content (Widgets)
                widgets_ref.render_content(
                    &mut self.shape_renderer,
                    &mut self.text_renderer,
                    &self.context,
                    &mut self.font_atlas,
                    w as u32,
                    h as u32,
                );

                // Flush base content (Shapes then Text)
                self.shape_renderer.record_commands(
                    command_buffer,
                    self.swapchain.extent().width,
                    self.swapchain.extent().height,
                    self.logical_width,
                    self.logical_height,
                );
                self.text_renderer.record_commands(
                    command_buffer,
                    self.swapchain.extent().width,
                    self.swapchain.extent().height,
                    self.logical_width,
                    self.logical_height,
                );

                // 2. Render Overlays
                widgets_ref.render_overlays(
                    &mut self.shape_renderer,
                    &mut self.text_renderer,
                    &self.context,
                    &mut self.font_atlas,
                    w as u32,
                    h as u32,
                );

                // Flush overlays
                self.shape_renderer.record_commands(
                    command_buffer,
                    self.swapchain.extent().width,
                    self.swapchain.extent().height,
                    self.logical_width,
                    self.logical_height,
                );
                self.text_renderer.record_commands(
                    command_buffer,
                    self.swapchain.extent().width,
                    self.swapchain.extent().height,
                    self.logical_width,
                    self.logical_height,
                );
            } else {
                // Test Pattern (fallback when no widgets set)
                let cx = w / 2.0;
                let cy = h / 2.0;

                // Blue button
                self.shape_renderer.draw_rect(
                    cx - 100.0,
                    cy + 50.0,
                    200.0,
                    60.0,
                    [0.2, 0.2, 0.8, 1.0],
                    10.0,
                );

                // Red circle
                self.shape_renderer
                    .draw_circle(cx, cy - 50.0, 40.0, [0.8, 0.2, 0.2, 1.0]);

                // Default Rect
                self.shape_renderer
                    .draw_rect(50.0, 50.0, 100.0, 100.0, [1.0, 1.0, 0.0, 1.0], 0.0);

                self.shape_renderer.record_commands(
                    command_buffer,
                    self.swapchain.extent().width,
                    self.swapchain.extent().height,
                    self.logical_width,
                    self.logical_height,
                );
                self.text_renderer.record_commands(
                    command_buffer,
                    self.swapchain.extent().width,
                    self.swapchain.extent().height,
                    self.logical_width,
                    self.logical_height,
                );
            }

            // FPS Overlay (always show)
            let fps_width = (self.current_fps / 60.0 * 100.0).clamp(0.0, 100.0);
            let color = if self.current_fps > 55.0 {
                [0.0, 1.0, 0.0, 1.0] // Green
            } else if self.current_fps > 45.0 {
                [1.0, 1.0, 0.0, 1.0] // Yellow
            } else {
                [1.0, 0.0, 0.0, 1.0] // Red
            };
            self.shape_renderer
                .draw_rect(10.0, h - 30.0, fps_width, 20.0, color, 0.0);

            // FPS Text
            let fps_text = format!(
                "FPS: {:.1} ({:.2}ms)",
                self.current_fps,
                self.get_avg_frame_time()
            );
            self.text_renderer.draw_text(
                &self.context,
                &mut self.font_atlas,
                &fps_text,
                10.0,
                h - 60.0,
                20.0,
                [1.0, 1.0, 1.0, 1.0],
            )?;

            // Record all rendering commands (Flush FPS)
            self.shape_renderer.record_commands(
                command_buffer,
                self.swapchain.extent().width,
                self.swapchain.extent().height,
                self.logical_width,
                self.logical_height,
            );
            self.text_renderer.record_commands(
                command_buffer,
                self.swapchain.extent().width,
                self.swapchain.extent().height,
                self.logical_width,
                self.logical_height,
            );

            device.cmd_end_render_pass(command_buffer);

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
        if let Some(fixed) = self.fixed_fps {
            self.current_fps = fixed;
            return;
        }

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

    /// Capture the current frame as an image
    pub fn capture_frame(&self) -> Result<RgbaImage> {
        unsafe {
            let device = self.context.device();
            device.device_wait_idle()?;

            let width = self.swapchain.extent().width;
            let height = self.swapchain.extent().height;
            let image_size = (width * height * 4) as u64;

            // 1. Create a transfer buffer (HOST_VISIBLE)
            let buffer_info = vk::BufferCreateInfo::default()
                .size(image_size)
                .usage(vk::BufferUsageFlags::TRANSFER_DST)
                .sharing_mode(vk::SharingMode::EXCLUSIVE);

            let buffer = device.create_buffer(&buffer_info, None)?;

            let mem_reqs = device.get_buffer_memory_requirements(buffer);
            let mem_type_index = self
                .context
                .find_memory_type(
                    mem_reqs.memory_type_bits,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
                .context("Failed to find suitable memory for capture buffer")?;

            let alloc_info = vk::MemoryAllocateInfo::default()
                .allocation_size(mem_reqs.size)
                .memory_type_index(mem_type_index);

            let memory = device.allocate_memory(&alloc_info, None)?;
            device.bind_buffer_memory(buffer, memory, 0)?;

            // 2. Copy from Swapchain Image to Buffer
            // We need a command buffer for this
            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(self.command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let cmd_buffer = device.allocate_command_buffers(&alloc_info)?[0];

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            device.begin_command_buffer(cmd_buffer, &begin_info)?;

            // Assume we want to capture the FIRST swapchain image for testing simplicity
            // In a real scenario, we would capture the just-rendered image index
            let src_image = self.swapchain.images()[0];

            let image_barrier = vk::ImageMemoryBarrier::default()
                .image(src_image)
                .src_access_mask(vk::AccessFlags::MEMORY_READ)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL) // Actually we are coming from render pass so it might be PRESENT_SRC_KHR
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier],
            );

            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });

            // Bind regions to avoid temporary value dropped while borrowed
            let regions = [region];
            device.cmd_copy_image_to_buffer(
                cmd_buffer,
                src_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                buffer,
                &regions,
            );

            // Barrier back to PRESENT_SRC
            let image_barrier_back = vk::ImageMemoryBarrier::default()
                .image(src_image)
                .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[image_barrier_back],
            );

            device.end_command_buffer(cmd_buffer)?;

            // Bind command buffers only once here
            let command_buffers_submit = [cmd_buffer];
            let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers_submit);

            let fence = device.create_fence(&vk::FenceCreateInfo::default(), None)?;
            device.queue_submit(self.context.graphics_queue(), &[submit_info], fence)?;
            device.wait_for_fences(&[fence], true, u64::MAX)?;

            // 3. Map memory and view as Image
            let ptr =
                device.map_memory(memory, 0, image_size, vk::MemoryMapFlags::empty())? as *const u8;
            let slice = std::slice::from_raw_parts(ptr, image_size as usize);

            // Swapchain is usually BGRA or BGR, we need RGBA.
            // Check surface format from swapchain
            let format = self.swapchain.format();

            let mut rgba_data = Vec::with_capacity(image_size as usize);

            if format == vk::Format::B8G8R8A8_SRGB || format == vk::Format::B8G8R8A8_UNORM {
                for chunk in slice.chunks(4) {
                    rgba_data.push(chunk[2]); // R
                    rgba_data.push(chunk[1]); // G
                    rgba_data.push(chunk[0]); // B
                    rgba_data.push(chunk[3]); // A
                }
            } else {
                // Assume RGBA
                rgba_data.extend_from_slice(slice);
            }

            device.unmap_memory(memory);
            device.destroy_fence(fence, None);
            device.free_command_buffers(self.command_pool, &[cmd_buffer]);
            device.destroy_buffer(buffer, None);
            device.free_memory(memory, None);

            RgbaImage::from_raw(width, height, rgba_data)
                .context("Failed to create RgbaImage from raw data")
        }
    }

    /// Resize the renderer resources (swapchain and framebuffers)
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        unsafe {
            let device = self.context.device();
            device.device_wait_idle()?;

            // 1. Destroy framebuffers
            for framebuffer in &self.framebuffers {
                device.destroy_framebuffer(*framebuffer, None);
            }
            self.framebuffers.clear();

            // 2. Cleanup old swapchain
            self.swapchain.cleanup(device);

            // 3. Create new swapchain
            self.swapchain = Swapchain::new(&self.context, width, height)?;

            // 4. Recreate framebuffers
            self.framebuffers =
                Self::create_framebuffers(&self.context, &self.swapchain, self.render_pass)?;

            // 5. Update dimensions
            self.logical_width = width as f32;
            self.logical_height = height as f32;

            Ok(())
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            // Wait for device to be idle before destroying resources
            let _ = self.context.device().device_wait_idle();

            // Destroy synchronization objects
            for semaphore in &self.render_finished_semaphores {
                self.context.device().destroy_semaphore(*semaphore, None);
            }
            for semaphore in &self.image_available_semaphores {
                self.context.device().destroy_semaphore(*semaphore, None);
            }
            for fence in &self.in_flight_fences {
                self.context.device().destroy_fence(*fence, None);
            }

            // Destroy framebuffers and render pass
            for framebuffer in &self.framebuffers {
                self.context
                    .device()
                    .destroy_framebuffer(*framebuffer, None);
            }
            self.context
                .device()
                .destroy_render_pass(self.render_pass, None);

            // ShapeRenderer drops itself

            // Destroy swapchain
            self.swapchain.cleanup(self.context.device());

            // Destroy command pool
            self.context
                .device()
                .destroy_command_pool(self.command_pool, None);
        }
    }
}
