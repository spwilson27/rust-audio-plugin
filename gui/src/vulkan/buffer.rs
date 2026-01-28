use super::VulkanContext;
use anyhow::Result;
use ash::vk;

/// Allocate a Vulkan buffer with the specified properties.
///
/// Returns the buffer and its allocated memory.
pub fn create_buffer(
    context: &VulkanContext,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let device = context.device();

    let buffer_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };

    let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let memory_type = find_memory_type(context, mem_requirements.memory_type_bits, properties)?;

    let alloc_info = vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(memory_type);

    let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None)? };

    unsafe {
        device.bind_buffer_memory(buffer, buffer_memory, 0)?;
    }

    Ok((buffer, buffer_memory))
}

/// Helper to find a suitable memory type index.
pub fn find_memory_type(
    context: &VulkanContext,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32> {
    let mem_properties = unsafe {
        context
            .instance()
            .get_physical_device_memory_properties(context.physical_device())
    };

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && (mem_properties.memory_types[i as usize].property_flags & properties) == properties
        {
            return Ok(i);
        }
    }

    anyhow::bail!("Failed to find suitable memory type")
}
