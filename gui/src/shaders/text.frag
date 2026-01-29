#version 450

layout(location = 0) in vec2 fragUV;
layout(location = 1) in vec4 fragColor;

layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

void main() {
    // Sample alpha from red channel (standard for single channel fonts)
    float alpha = texture(texSampler, fragUV).r;
    
    // Discard fully transparent (optional, but good for depth if used)
    if (alpha <= 0.0) discard;
    
    // Premultiplied alpha
    float outputAlpha = fragColor.a * alpha;
    outColor = vec4(fragColor.rgb * outputAlpha, outputAlpha);
}
