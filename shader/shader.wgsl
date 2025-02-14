// Fundamental Cook-Torrance PBR Shader

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

// View
struct ViewUniform {
    @location(0) projection: mat4x4<f32>,
    @location(1) position: vec3f
}
@group(1) @binding(0)
var<uniform> view: ViewUniform;

// Model
struct ModelUniform {
    @location(0) model: mat4x4<f32>
}
@group(2) @binding(0)
var<uniform> model: ModelUniform;

// Lights
const MAX_POINT_LIGHTS : u32 = 32u;
const MAX_DIR_LIGHTS : u32 = 16u;
struct PointLight {
    @location(0) position: vec3f
}
struct DirLight {
    @location(0) direction: vec3f,
}

@group(3) @binding(0)
var<uniform> point_lights: array<PointLight, MAX_POINT_LIGHTS>;
@group(3) @binding(1)
var<uniform> dir_lights: array<DirLight, MAX_DIR_LIGHTS>;


struct VertexInput {
    @location(0) position: vec3f,
    @location(1) texcoord: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec4f, 
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) position: vec3f,
    @location(1) texcoord: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec4f,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    var position = model.model * vec4(in.position, 1.0);
    out.clip_position = view.projection * position;
    out.texcoord = in.texcoord;
    out.position = position.xyz;
    out.normal = in.normal;
    out.tangent = in.tangent;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // Normal Mapping
    var normal = normalize(in.normal);
    var tangent = normalize(in.tangent.xyz);
    var bitangent = normalize(cross(normal, tangent)) * in.tangent.w;
    var pbr_normal = textureSample(t_normal, s_normal, in.texcoord).xyz - vec3(0.5);
    var tbn = mat3x3(tangent, bitangent, normal);
    normal = normalize(tbn * pbr_normal);


    return textureSample(t_albedo, s_albedo, in.texcoord);
}