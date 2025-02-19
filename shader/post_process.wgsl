@group(0) @binding(0)
var t_screen: texture_2d<f32>;
@group(0) @binding(1)
var s_screen: sampler;
@group(1) @binding(0)
var<uniform> screen_size: vec2<i32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) texcoord: vec2f,
}


@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var out: VertexOutput;
    var position = array<vec2f, 4>(vec2f(-1.0, -1.0), vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0));
    var texcoord = array<vec2f, 4>(vec2f(0.0, 1.0), vec2f(0.0, 0.0), vec2f(1.0, 1.0), vec2f(1.0, 0.0));
    var indices = array<u32, 6>(0, 1, 2, 1, 2, 3);
    out.clip_position = vec4f(position[indices[index] ], 0.0, 1.0);
    out.texcoord = texcoord[indices[index] ];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // Reinhard Tonemapping
    var color = textureSample(t_screen, s_screen, in.texcoord).xyz;

    return vec4f(color, 1.0);
}

