#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec2 inUV;
layout(location = 2) in vec4 inColor;

layout(location = 0) out vec2 fragUV;
layout(location = 1) out vec4 fragColor;

layout(push_constant) uniform PushConstants {
    vec2 screenSize;
} push;

void main() {
    // Map pixel coordinates to Vulkan NDC (-1 to 1)
    // 0,0 is top-left
    vec2 glPos = (inPosition / push.screenSize) * 2.0 - 1.0;
    
    gl_Position = vec4(glPos, 0.0, 1.0);
    fragUV = inUV;
    fragColor = inColor;
}
