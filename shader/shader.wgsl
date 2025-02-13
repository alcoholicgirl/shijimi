// Fundamental PBR Shader
// PBR Textures

// Albedo
@group(0) @binding(0)
var t_albedo: texture_2d<f32>;
@group(0) @binding(1)
var s_albedo: sampler;
@group(0) @binding(2)
var<uniform> m_albedo: vec4f;

// Normal
@group(0) @binding(3)
var t_normal: texture_2d<f32>;
@group(0) @binding(4)
var s_normal: sampler;
@group(0) @binding(5)
var<uniform> m_normal: vec4f;

// Metallic
@group(0) @binding(6)
var t_metallic: texture_2d<f32>;
@group(0) @binding(7)
var s_metallic: sampler;
@group(0) @binding(8)
var<uniform> m_metallic: vec4f;

// Roughness
@group(0) @binding(9)
var t_roughness: texture_2d<f32>;
@group(0) @binding(10)
var s_roughness: sampler;
@group(0) @binding(11)
var<uniform> m_roughness: vec4f;

// AO
@group(0) @binding(12)
var t_ao: texture_2d<f32>;
@group(0) @binding(13)
var s_ao: sampler;
@group(0) @binding(14)
var<uniform> m_ao: vec4f;

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
    @location(2) normal: vec3f,
    @location(3) tangent: vec4f, 
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
    return textureSample(t_albedo, s_albedo, in.texcoord);
}