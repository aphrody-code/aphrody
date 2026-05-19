struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct InstanceInput {
    @location(2) position: vec2<f32>,
    @location(3) size: vec2<f32>,
    @location(4) tex_src: vec4<f32>, // x, y, w, h
    @location(5) color: vec4<f32>,
    @location(6) bg_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
}

struct Uniforms {
    screen_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    // Convert screen coordinates to NDC [-1, 1]
    let pos_px = instance.position + model.position * instance.size;
    let pos_ndc = vec2<f32>(
        (pos_px.x / uniforms.screen_size.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / uniforms.screen_size.y) * 2.0
    );

    out.clip_position = vec4<f32>(pos_ndc, 0.0, 1.0);

    // Calculate actual texture coordinates for this vertex
    let tex_x = instance.tex_src.x + model.tex_coords.x * instance.tex_src.z;
    let tex_y = instance.tex_src.y + model.tex_coords.y * instance.tex_src.w;
    out.tex_coords = vec2<f32>(tex_x, tex_y);

    out.color = instance.color;
    out.bg_color = instance.bg_color;
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Read the alpha mask from the atlas
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    // Lerp between background and foreground based on glyph alpha (which is usually red channel or alpha channel in swash)
    // Here we assume it's stored in the red channel for simplicity.
    let alpha = tex_color.r;
    
    // Simple blending
    let final_color = mix(in.bg_color, in.color, alpha);
    return final_color;
}
