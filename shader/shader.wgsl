// Textures
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// Camera
struct CameraUniform {
    @location(0) view_proj: mat4x4<f32> 
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

// Model
struct ModelUniform {
    @location(0) model: mat4x4<f32>
}
@group(2) @binding(0)
var<uniform> model: ModelUniform;


struct VertexInput {
    @location(0) position: vec3f,
    @location(1) texcoord: vec2f,
    // @location(2) normal: vec3f,
    // @location(3) tangent: vec4f, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) texcoord: vec2f,
};

@vertex
fn vs_main(
    vertex: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * model.model * vec4(vertex.position, 1.0);
    out.texcoord = vertex.texcoord;
    return out;
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(t_diffuse, s_diffuse, in.texcoord);
}