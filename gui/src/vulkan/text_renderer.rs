use anyhow::{Context, Result};
use ash::vk;
use fontdue::{Font, FontSettings};
use guillotiere::{AtlasAllocator, Size};
use std::collections::HashMap;

use std::mem;

use super::{buffer, texture, VulkanContext};
use crate::fonts;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TextPushConstants {
    pub screen_size: [f32; 2],
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct GlyphKey {
    c: char,
    px: u32,
}

/// Manages a dynamic texture atlas for font glyphs.
///
/// Uses `guillotiere` to pack glyphs into a single `R8_UNORM` texture.
/// Uploads rasterized glyphs on demand via a staging buffer.
pub struct FontAtlas {
    font: Font,
    allocator: AtlasAllocator,
    /// Cache of mapped glyphs: (char, size_px) -> (UV Min, UV Max)
    cache: HashMap<GlyphKey, ([f32; 2], [f32; 2])>,

    // Vulkan Resources
    device: ash::Device,
    image: vk::Image,
    _memory: vk::DeviceMemory,
    view: vk::ImageView,
    sampler: vk::Sampler,

    width: u32,
    height: u32,

    // Command resources for uploads
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
}

impl FontAtlas {
    /// Initialize the Atlas with a fixed size (e.g. 1024x1024).
    pub fn new(
        context: &VulkanContext,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> Result<Self> {
        let font_data = fonts::ROBOTO_REGULAR;
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| anyhow::anyhow!("Failed to load font: {}", e))?;

        let width = 1024;
        let height = 1024;
        let allocator = AtlasAllocator::new(Size::new(width as i32, height as i32));

        // Create texture (R8_UNORM)
        let (image, memory) = texture::create_image(
            context,
            width,
            height,
            vk::Format::R8_UNORM,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let device = context.device();
        let view = texture::create_image_view(device, image, vk::Format::R8_UNORM)?;
        let sampler = texture::create_sampler(device)?;

        // Initialize texture with transparent black to prevent garbage artifacts
        // 1. Transition to TRANSFER_DST
        texture::transition_image_layout(
            device,
            command_pool,
            graphics_queue,
            image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;

        // 2. Clear to transparent
        let clear_color = vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 0.0],
        };
        texture::clear_image(
            device,
            command_pool,
            graphics_queue,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            clear_color,
        )?;

        // 3. Transition to SHADER_READ
        texture::transition_image_layout(
            device,
            command_pool,
            graphics_queue,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        Ok(Self {
            font,
            allocator,
            cache: HashMap::new(),
            device: device.clone(),
            image,
            _memory: memory,
            view,
            sampler,
            width,
            height,
            command_pool,
            graphics_queue,
        })
    }

    /// Retrieve UV coordinates for a glyph.
    ///
    /// If the glyph is not in the atlas, it is rasterized and uploaded immediately.
    /// This involves a GPU wait/upload, so it should ideally be batched or pre-warmed,
    /// but for this implementation it happens on-demand per frame if missing.
    pub fn get_glyph_uv(
        &mut self,
        context: &VulkanContext,
        c: char,
        px: f32,
    ) -> Result<Option<([f32; 2], [f32; 2])>> {
        let key = GlyphKey { c, px: px as u32 }; // Round size for caching

        if let Some(uvs) = self.cache.get(&key) {
            return Ok(Some(*uvs));
        }

        // Rasterize
        let (metrics, bitmap) = self.font.rasterize(c, px);
        if bitmap.is_empty() {
            return Ok(None); // Space or invisible
        }

        let width = metrics.width as i32;
        let height = metrics.height as i32;

        // Allocate in atlas
        // Add 1px padding
        let padding = 1;
        let alloc_size = Size::new(width + padding * 2, height + padding * 2);

        let alloc = match self.allocator.allocate(alloc_size) {
            Some(a) => a,
            None => {
                // Atlas full? Clear and restart?
                // For simplified impl, just warn and return None (glyph missing).
                tracing::warn!("Font Atlas Full!");
                return Ok(None);
            }
        };

        // Upload to texture
        // We need a staging buffer
        let buffer_size = (width * height) as u64; // 1 byte per pixel
        let (staging_buffer, staging_memory) = buffer::create_buffer(
            context,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let device = context.device();
        unsafe {
            let ptr =
                device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?
                    as *mut u8;
            ptr.copy_from_nonoverlapping(bitmap.as_ptr(), bitmap.len());
            device.unmap_memory(staging_memory);
        }

        // Copy to image
        // 1. Transition to TRANSFER_DST
        texture::transition_image_layout(
            device,
            self.command_pool,
            self.graphics_queue,
            self.image,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;

        // 2. Copy buffer
        let offset_x = alloc.rectangle.min.x + padding;
        let offset_y = alloc.rectangle.min.y + padding;

        texture::copy_buffer_to_image(
            device,
            self.command_pool,
            self.graphics_queue,
            staging_buffer,
            self.image,
            width as u32,
            height as u32,
            offset_x,
            offset_y,
        )?;

        // 3. Transition back to SHADER_READ
        texture::transition_image_layout(
            device,
            self.command_pool,
            self.graphics_queue,
            self.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        // Cleanup staging
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        }

        // Calculate UVs
        let atlas_w = self.width as f32;
        let atlas_h = self.height as f32;

        let uv_min = [offset_x as f32 / atlas_w, offset_y as f32 / atlas_h];
        let uv_max = [
            (offset_x + width) as f32 / atlas_w,
            (offset_y + height) as f32 / atlas_h,
        ];

        let uvs = (uv_min, uv_max);
        self.cache.insert(key, uvs);

        Ok(Some(uvs))
    }

    /// Measure the width of a string string in pixels.
    pub fn measure_text(&self, text: &str, size: f32) -> f32 {
        let mut width = 0.0;
        for c in text.chars() {
            let metrics = self.font.metrics(c, size);
            width += metrics.advance_width;
        }
        width
    }

    /// Find the character index at a given X position (for mouse clicks).
    pub fn get_char_index_at_width(&self, text: &str, size: f32, target_x: f32) -> usize {
        let mut current_x = 0.0;
        for (i, c) in text.chars().enumerate() {
            let metrics = self.font.metrics(c, size);
            let advance = metrics.advance_width;

            // If the click is within the left half of this char, return this index.
            // If right half, continue (will return next index or end).
            // Actually, standard behavior is usually nearest boundary.

            if target_x < current_x + advance / 2.0 {
                return i;
            }

            current_x += advance;
        }
        text.len()
    }

    /// Get usage metrics for a single glyph (advance width)
    pub fn get_glyph_advance(&self, c: char, size: f32) -> f32 {
        self.font.metrics(c, size).advance_width
    }
}

impl Drop for FontAtlas {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self._memory, None);
        }
    }
}

// ... Reimplement logic with Resources struct ...
// Actually, let's just add `device: ash::Device` to FontAtlas.

/// Renders text using textured quads sourced from a `FontAtlas`.
pub struct TextRenderer {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet, // Single set for the atlas

    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    vertex_count: usize,
    max_vertex_count: usize,
    host_mapped_memory: *mut TextVertex,
}

impl TextRenderer {
    pub fn new(
        context: &VulkanContext,
        render_pass: vk::RenderPass,
        atlas: &FontAtlas,
    ) -> Result<Self> {
        let device = context.device().clone();

        // ... (Shader creation similar to ShapeRenderer) ...
        // Need to load text.vert.spv and text.frag.spv
        let vert_code = include_bytes!(concat!(env!("OUT_DIR"), "/text.vert.spv"));
        let frag_code = include_bytes!(concat!(env!("OUT_DIR"), "/text.frag.spv"));

        let vert_module = super::shape_renderer::create_shader_module(&device, vert_code)?;
        let frag_module = super::shape_renderer::create_shader_module(&device, frag_code)?;

        let main_function_name = c"main";

        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vert_module)
                .name(main_function_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(frag_module)
                .name(main_function_name),
        ];

        // Descriptor Set Layout (For Texture)
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(std::slice::from_ref(&binding));

        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        // Descriptor Pool
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
        };
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(std::slice::from_ref(&pool_size))
            .max_sets(1);

        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        // Allocate Descriptor Set
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&descriptor_set_layout));

        let descriptor_set = unsafe { device.allocate_descriptor_sets(&alloc_info)?[0] };

        // Update Descriptor Set with Atlas
        let image_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(atlas.view)
            .sampler(atlas.sampler);

        let write_descriptor_set = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&image_info));

        unsafe {
            device.update_descriptor_sets(std::slice::from_ref(&write_descriptor_set), &[]);
        }

        // Pipeline Layout
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: mem::size_of::<TextPushConstants>() as u32,
        };
        let layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(std::slice::from_ref(&descriptor_set_layout))
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));

        let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

        // Vertex Input
        let binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<TextVertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];

        let attribute_descriptions = [
            // Position
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 0,
            },
            // UV
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: 8,
            },
            // Color
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            },
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        // ... Input Assembly, Viewport, Rasterizer, Multisample, Blend ...
        // (Copy from ShapeRenderer but enable blending)

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // Standard Alpha Blending
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        };

        let color_blend_attachments = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|e| e.1)
                .context("Failed to create text pipeline")?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        // Vertex Buffer
        let max_vertex_count = 10000;
        let buffer_size = (mem::size_of::<TextVertex>() * max_vertex_count) as u64;

        let (vertex_buffer, vertex_memory) = buffer::create_buffer(
            context,
            buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let host_mapped_memory = unsafe {
            device.map_memory(vertex_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?
                as *mut TextVertex
        };

        Ok(Self {
            device,
            pipeline_layout,
            pipeline,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            vertex_buffer,
            vertex_memory,
            vertex_count: 0,
            max_vertex_count,
            host_mapped_memory,
        })
    }

    pub fn begin(&mut self) {
        self.vertex_count = 0;
    }

    /// Draw a string of text.
    ///
    /// Appends quads to the internal vertex buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text(
        &mut self,
        context: &VulkanContext,
        atlas: &mut FontAtlas,
        text: &str,
        x: f32,
        y: f32,
        size: f32,
        color: [f32; 4],
    ) -> Result<()> {
        // Use fontdue::layout for proper positioning ideally, but simple advance for now.

        let mut cx = x;
        let cy = y;

        for c in text.chars() {
            // 1. Get UVs (mut borrow of atlas)
            let uvs_opt = atlas.get_glyph_uv(context, c, size)?;

            if let Some((uv_min, uv_max)) = uvs_opt {
                // 2. Get Metrics (immut borrow of atlas.font)
                // We need to re-borrow atlas here.
                // Since get_glyph_uv is finished, we can borrow atlas again.
                let metrics = atlas.font.metrics(c, size);

                let w = metrics.width as f32;
                let h = metrics.height as f32;

                // Standard Top-Left origin logic with Y-down:
                // Baseline is not easily known without Layout.
                // fontdue bounds are relative to origin.
                // "xmin is the left side bearing"
                // "ymin is the bottom side bearing" (positive y is up in fontdue)
                // "height" is height of bounding box.

                // Render at baseline (cy)
                // In Y-down system:
                // Top of glyph = cy - (height + ymin)
                // This assumes ymin is distance from baseline to bottom of glyph (positive up).
                // e.g. for 'g', ymin might be negative.
                // If ymin = -5, height = 20. Top = cy - (20 + (-5)) = cy - 15.
                // Bottom = cy - (-5) = cy + 5.

                let top = cy - (metrics.ymin as f32 + metrics.height as f32);
                let box_x = cx + metrics.xmin as f32;
                let box_y = top;
                let box_w = w;
                let box_h = h;

                self.push_quad(box_x, box_y, box_w, box_h, uv_min, uv_max, color);

                cx += metrics.advance_width;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn push_quad(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        color: [f32; 4],
    ) {
        if self.vertex_count + 6 > self.max_vertex_count {
            return;
        }

        // Quad
        let p0 = [x, y];
        let p1 = [x + w, y];
        let p2 = [x + w, y + h];
        let p3 = [x, y + h];

        let t0 = [uv_min[0], uv_min[1]];
        let t1 = [uv_max[0], uv_min[1]];
        let t2 = [uv_max[0], uv_max[1]];
        let t3 = [uv_min[0], uv_max[1]];

        let v0 = TextVertex {
            position: p0,
            uv: t0,
            color,
        };
        let v1 = TextVertex {
            position: p1,
            uv: t1,
            color,
        };
        let v2 = TextVertex {
            position: p2,
            uv: t2,
            color,
        };
        let v3 = TextVertex {
            position: p3,
            uv: t3,
            color,
        };

        unsafe {
            let ptr = self.host_mapped_memory.add(self.vertex_count);
            *ptr.add(0) = v0;
            *ptr.add(1) = v1;
            *ptr.add(2) = v2;
            *ptr.add(3) = v2;
            *ptr.add(4) = v3;
            *ptr.add(5) = v0;
        }

        self.vertex_count += 6;
    }

    pub fn record_commands(
        &self,
        command_buffer: vk::CommandBuffer,
        viewport_width: u32,
        viewport_height: u32,
        logical_width: f32,
        logical_height: f32,
    ) {
        if self.vertex_count == 0 {
            return;
        }

        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: viewport_width as f32,
                height: viewport_height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);

            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: viewport_width,
                    height: viewport_height,
                },
            };
            self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

            // Push Constants
            let push_constants = TextPushConstants {
                screen_size: [logical_width, logical_height],
            };
            let push_ptr = &push_constants as *const TextPushConstants as *const u8;
            let push_slice =
                std::slice::from_raw_parts(push_ptr, mem::size_of::<TextPushConstants>());

            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_slice,
            );

            // Bind Descriptor Set
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );

            // Bind Vertex Buffer
            let buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);

            self.device
                .cmd_draw(command_buffer, self.vertex_count as u32, 1, 0, 0);
        }
    }
}

impl Drop for TextRenderer {
    fn drop(&mut self) {
        unsafe {
            self.device.unmap_memory(self.vertex_memory);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_memory, None);

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
