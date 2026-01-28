use anyhow::{Context, Result};
use ash::vk;

use std::mem;

use super::VulkanContext;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShapeVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub size: [f32; 2],
    pub radius: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PushConstants {
    pub screen_size: [f32; 2],
}

pub struct ShapeRenderer {
    device: ash::Device,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    vertex_buffer: vk::Buffer,
    vertex_memory: vk::DeviceMemory,
    vertex_count: usize,
    max_vertex_count: usize,
    host_mapped_memory: *mut ShapeVertex,
}

impl ShapeRenderer {
    pub fn new(context: &VulkanContext, render_pass: vk::RenderPass) -> Result<Self> {
        let device = context.device().clone();

        // 1. Create Shaders
        let vert_code = include_bytes!(concat!(env!("OUT_DIR"), "/shape.vert.spv"));
        let frag_code = include_bytes!(concat!(env!("OUT_DIR"), "/shape.frag.spv"));

        let vert_module = create_shader_module(&device, vert_code)?;
        let frag_module = create_shader_module(&device, frag_code)?;

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

        // 2. Vertex Input
        let binding_descriptions = [vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<ShapeVertex>() as u32,
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
            // Size
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 3,
                format: vk::Format::R32G32_SFLOAT,
                offset: 32,
            },
            // Radius
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 4,
                format: vk::Format::R32_SFLOAT,
                offset: 40,
            },
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions);

        // 3. Input Assembly
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // 4. Viewport (Dynamic)
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        // 5. Rasterizer
        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        // 6. Multisampling (Disabled for SDF)
        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // 7. Color Blending
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::TRUE,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
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

        // 8. Pipeline Layout
        let push_constant_range = vk::PushConstantRange {
            stage_flags: vk::ShaderStageFlags::VERTEX,
            offset: 0,
            size: mem::size_of::<PushConstants>() as u32,
        };

        let push_constant_ranges_array = [push_constant_range];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(&push_constant_ranges_array);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        // 9. Graphics Pipeline
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
                .context("Failed to create graphics pipeline")?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        // 10. Vertex Buffer (Dynamic)
        let max_vertex_count = 10000;
        let buffer_size = (mem::size_of::<ShapeVertex>() * max_vertex_count) as u64;

        let (vertex_buffer, vertex_memory) = super::buffer::create_buffer(
            context,
            buffer_size,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        // Persistent mapping
        let host_mapped_memory = unsafe {
            device.map_memory(vertex_memory, 0, buffer_size, vk::MemoryMapFlags::empty())?
                as *mut ShapeVertex
        };

        Ok(Self {
            device,
            pipeline_layout,
            pipeline,
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

    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4], radius: f32) {
        if self.vertex_count + 6 > self.max_vertex_count {
            // TODO: Flush or resize
            return;
        }

        // Local coords for SDF (-1 to 1)
        // 0: -1, -1 (TL)
        // 1:  1, -1 (TR)
        // 2:  1,  1 (BR)
        // 3: -1,  1 (BL)
        //
        // Indices (Triangles):
        // 0, 1, 2
        // 2, 3, 0

        let size = [w, h];
        let p0 = [x, y];
        let p1 = [x + w, y];
        let p2 = [x + w, y + h];
        let p3 = [x, y + h];

        let uv0 = [-1.0, -1.0];
        let uv1 = [1.0, -1.0];
        let uv2 = [1.0, 1.0];
        let uv3 = [-1.0, 1.0];

        let v0 = ShapeVertex {
            position: p0,
            uv: uv0,
            color,
            size,
            radius,
        };
        let v1 = ShapeVertex {
            position: p1,
            uv: uv1,
            color,
            size,
            radius,
        };
        let v2 = ShapeVertex {
            position: p2,
            uv: uv2,
            color,
            size,
            radius,
        };
        let v3 = ShapeVertex {
            position: p3,
            uv: uv3,
            color,
            size,
            radius,
        };

        unsafe {
            let ptr = self.host_mapped_memory.add(self.vertex_count);
            // Triangle 1
            *ptr.add(0) = v0;
            *ptr.add(1) = v1;
            *ptr.add(2) = v2;
            // Triangle 2
            *ptr.add(3) = v2;
            *ptr.add(4) = v3;
            *ptr.add(5) = v0;
        }

        self.vertex_count += 6;
    }

    pub fn draw_circle(&mut self, cx: f32, cy: f32, r: f32, color: [f32; 4]) {
        // Circle is just a rounded rect with w=h=2r and radius=r
        let size = r * 2.0;
        self.draw_rect(cx - r, cy - r, size, size, color, r);
    }

    pub fn record_commands(
        &self,
        command_buffer: vk::CommandBuffer,
        screen_width: u32,
        screen_height: u32,
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

            // Set dynamic viewport
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: screen_width as f32,
                height: screen_height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);

            // Set dynamic scissor
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: screen_width,
                    height: screen_height,
                },
            };
            self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

            let push_constants = PushConstants {
                screen_size: [screen_width as f32, screen_height as f32],
            };

            // Cast struct to byte slice
            let push_ptr = &push_constants as *const PushConstants as *const u8;
            let push_slice = std::slice::from_raw_parts(push_ptr, mem::size_of::<PushConstants>());

            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_slice,
            );

            let buffers = [self.vertex_buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &buffers, &offsets);

            self.device
                .cmd_draw(command_buffer, self.vertex_count as u32, 1, 0, 0);
        }
    }
}

impl Drop for ShapeRenderer {
    fn drop(&mut self) {
        unsafe {
            self.device.unmap_memory(self.vertex_memory);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_memory, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

pub fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule> {
    let shader_module_create_info = vk::ShaderModuleCreateInfo {
        code_size: code.len(),
        p_code: code.as_ptr() as *const u32,
        ..Default::default()
    };

    unsafe { Ok(device.create_shader_module(&shader_module_create_info, None)?) }
}
