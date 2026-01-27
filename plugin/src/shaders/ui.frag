#version 450

// Fragment shader for UI rendering
// Samples from SVG-rasterized texture and applies alpha blending

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

void main() {
    // Sample the texture
    vec4 texColor = texture(texSampler, fragTexCoord);
    
    // Output with premultiplied alpha for proper blending
    // (Required for macOS transparency with VK_COMPOSITE_ALPHA_POST_MULTIPLIED_BIT_KHR)
    outColor = texColor;
}
