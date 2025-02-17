@group(0) @binding(0)
var t_screen: texture_2d<f32>;
@group(0) @binding(1)
var s_screen: sampler;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) texcoord: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) texcoord: vec2f,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out : VertexOutput;
    out.clip_position = vec4(in.position, 1.0);
    out.texcoord = in.texcoord;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let uv = in.texcoord;
    var color = textureSample(t_screen, s_screen, uv);
    return color;
}
