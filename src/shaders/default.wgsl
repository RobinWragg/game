override LIGHTING_ENABLED = false;

struct VertInput {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
}

struct VertToFrag {
    @builtin(position) pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct Uniform {
    matrix: mat4x4<f32>,
    color: vec4<f32>,
}
@group(0) @binding(0)
var<uniform> uniform: Uniform;

@vertex
fn vs_main(@builtin(vertex_index) vert_index: u32, vert: VertInput) -> VertToFrag {
    var out: VertToFrag;
    out.pos = uniform.matrix * vec4<f32>(vert.pos, 1.0);
    out.normal = vert.normal;
    out.color = srgb_to_linear(vert.color);

    // Warn about out of Z bounds.
    if out.pos.z < 0.0 {
        out.color.x  = 1.0;
        out.color.y  = 0.0;
        out.color.z  = 0.0;
        out.pos.z = 0.001;
    } else if out.pos.z > 1.0 {
        out.color.x  = 0.0;
        out.color.y  = 0.0;
        out.color.z  = 1.0;
        out.pos.z = 0.001;
    }

    out.uv = vert.uv;
    return out;
}

@group(1) @binding(0)
var texture_view: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;

@fragment
fn fs_main(in: VertToFrag) -> @location(0) vec4<f32> {
    let tex_color = textureSample(texture_view, texture_sampler, in.uv);

    let pre_light_color = tex_color * in.color * srgb_to_linear(uniform.color);

    if LIGHTING_ENABLED {
        let light = dot(in.normal, normalize(vec3<f32>(0.5, -1.0, 0.2))) / 2.0 + 0.5;
        let post_light_color = vec4<f32>(pre_light_color.rgb * light, pre_light_color.a);
        return post_light_color;
    } else {
        return pre_light_color;
    }
}

fn less_than(a: vec3<f32>, b: vec3<f32>) -> vec3<bool> {
    return vec3<bool>(a.x < b.x, a.y < b.y, a.z < b.z);
}

fn bool_to_f32(b: vec3<bool>) -> vec3<f32> {
    var out = vec3<f32>(0.0, 0.0, 0.0);
    if b.x { out.x = 1.0; }
    if b.y { out.y = 1.0; }
    if b.z { out.z = 1.0; }
    return out;
}

fn linear_to_srgb(linear: vec4<f32>) -> vec4<f32> {
    let cutoff: vec3<f32> = bool_to_f32(less_than(linear.rgb, vec3<f32>(0.0031308)));
    let higher: vec3<f32> = vec3<f32>(1.055) * pow(linear.rgb, vec3<f32>(1. / 2.4)) - vec3<f32>(0.055);
    let lower: vec3<f32> = linear.rgb * vec3<f32>(12.92);
    return vec4<f32>(mix(higher, lower, cutoff), linear.a);
}

fn srgb_to_linear(sRGB: vec4<f32>) -> vec4<f32> {
    let cutoff: vec3<f32> = bool_to_f32(less_than(sRGB.rgb, vec3<f32>(0.04045)));
    let higher: vec3<f32> = pow((sRGB.rgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    let lower: vec3<f32> = sRGB.rgb / vec3<f32>(12.92);
    return vec4<f32>(mix(higher, lower, cutoff), sRGB.a);
}