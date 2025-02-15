
// View
struct ViewUniform {
    @location(0) view_proj: mat4x4<f32>,
    @location(1) view_pos: vec3f
}
@group(0) @binding(0)
var<uniform> view: ViewUniform;

// Model
struct ModelUniform {
    @location(0) model: mat4x4<f32>
}
@group(1) @binding(0)
var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) texcoord: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec4f, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = view.view_proj * model.model * vec4(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return vec4f(0.0);
}