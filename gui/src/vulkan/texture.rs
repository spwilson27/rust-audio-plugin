use super::VulkanContext;
use anyhow::Result;
use ash::vk;

pub fn create_image(
    context: &VulkanContext,
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    let device = context.device();

    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .samples(vk::SampleCountFlags::TYPE_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let image = unsafe { device.create_image(&image_info, None)? };

    let mem_requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory_type =
        super::buffer::find_memory_type(context, mem_requirements.memory_type_bits, properties)?;

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type);

    let image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };

    unsafe {
        device.bind_image_memory(image, image_memory, 0)?;
    }

    Ok((image, image_memory))
}

pub fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView> {
    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(subresource_range);

    unsafe { Ok(device.create_image_view(&view_info, None)?) }
}

pub fn create_sampler(device: &ash::Device) -> Result<vk::Sampler> {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .anisotropy_enable(false)
        .max_anisotropy(1.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR);

    unsafe { Ok(device.create_sampler(&sampler_info, None)?) }
}

pub fn transition_image_layout(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<()> {
    let command_buffer = begin_single_time_commands(device, command_pool)?;

    let (src_access_mask, dst_access_mask, src_stage, dst_stage) = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        (vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
            vk::AccessFlags::SHADER_READ,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            vk::AccessFlags::empty(),
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        _ => anyhow::bail!("Unsupported layout transition!"),
    };

    let subresource_range = vk::ImageSubresourceRange::default()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1);

    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(subresource_range)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask);

    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        );
    }

    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;

    Ok(())
}

pub fn copy_buffer_to_image(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    buffer: vk::Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
) -> Result<()> {
    let command_buffer = begin_single_time_commands(device, command_pool)?;

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
        .image_offset(vk::Offset3D { x, y, z: 0 })
        .image_extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        });

    unsafe {
        device.cmd_copy_buffer_to_image(
            command_buffer,
            buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );
    }

    end_single_time_commands(device, command_pool, graphics_queue, command_buffer)?;

    Ok(())
}

fn begin_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
) -> Result<vk::CommandBuffer> {
    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool)
        .command_buffer_count(1);

    let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };

    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe {
        device.begin_command_buffer(command_buffer, &begin_info)?;
    }

    Ok(command_buffer)
}

fn end_single_time_commands(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    command_buffer: vk::CommandBuffer,
) -> Result<()> {
    unsafe {
        device.end_command_buffer(command_buffer)?;

        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);

        device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null())?;
        device.queue_wait_idle(graphics_queue)?;

        device.free_command_buffers(command_pool, &command_buffers);
    }

    Ok(())
}
