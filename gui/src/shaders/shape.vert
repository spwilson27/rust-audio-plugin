#version 450

layout(location = 0) in vec2 inPosition; // Screen space (pixels)
layout(location = 1) in vec2 inUV;       // Local SDF space (-1 to +1)
layout(location = 2) in vec4 inColor;
layout(location = 3) in vec2 inSize;     // Size in pixels (width, height)
layout(location = 4) in float inRadius;  // Corner radius in pixels

layout(location = 0) out vec2 outUV;
layout(location = 1) out vec4 outColor;
layout(location = 2) out vec2 outSize;
layout(location = 3) out float outRadius;

layout(push_constant) uniform PushConstants {
    vec2 screen_size;
} push;

void main() {
    outUV = inUV;
    outColor = inColor;
    outSize = inSize;
    outRadius = inRadius;

    // Convert pixel position [0, W], [0, H] to NDC [-1, 1]
    // X: (pos.x / W) * 2 - 1
    // Y: (pos.y / H) * 2 - 1
    // Y-flip: Vulkan Y is down, but screen coords usually top-left is 0,0.
    // Vulkan NDC: -1,-1 is top-left.
    // (0,0) -> -1,-1. (W,H) -> 1,1.
    
    vec2 normalized = (inPosition / push.screen_size) * 2.0 - 1.0;
    gl_Position = vec4(normalized, 0.0, 1.0);
}
