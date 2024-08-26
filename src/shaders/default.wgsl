// Vertex shader

struct VertInput {
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertToFrag {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> matrix: mat4x4<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vert_index: u32, vert: VertInput) -> VertToFrag {
    var out: VertToFrag;
    out.pos = matrix * vec4<f32>(vert.pos.x, vert.pos.y, 0.0, 1.0);
    out.color = vert.color;
    out.uv = vert.uv;
    return out;
}

// Fragment shader

@group(1) @binding(0)
var texture_view: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertToFrag) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture_view, texture_sampler, in.uv);
    let vert_color = in.color;
    return tex_color * vert_color;
}