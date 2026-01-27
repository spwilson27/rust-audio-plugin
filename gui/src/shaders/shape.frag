#version 450

layout(location = 0) in vec2 inUV;
layout(location = 1) in vec4 inColor;
layout(location = 2) in vec2 inSize;    
layout(location = 3) in float inRadius;

layout(location = 0) out vec4 outFragColor;

void main() {
    // Current pixel pos relative to center, in pixels
    // inUV is (-1 to 1). half_size is pixel extent from center.
    vec2 half_size = inSize / 2.0;    
    vec2 pos = inUV * half_size;

    // SDF for rounded box
    // d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius
    // q = abs(p) - (b - radius)
    
    // Effective box size for SDF calculation (b in formula)
    // is half_size reduced by radius corner.
    // Clamp radius to min(half_size.x, half_size.y) to handle circles safely
    float r = min(inRadius, min(half_size.x, half_size.y));
    
    vec2 q = abs(pos) - (half_size - vec2(r));
    float dist = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;

    // Antialiasing
    // smoothstep(edge - smoothing, edge + smoothing, -dist)
    // We render solid inside, so dist <= 0 is inside.
    // We want alpha 1 inside, 0 outside.
    // Boundary is at dist = 0.
    // Change over 1 pixel width.
    // fwidth gives change in dist per pixel.
    float delta = fwidth(dist);
    float alpha = 1.0 - smoothstep(-delta, delta, dist);

    outFragColor = vec4(inColor.rgb, inColor.a * alpha);
}
