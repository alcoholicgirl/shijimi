// Fundamental Cook-Torrance PBR Shader

// PBR Textures
const PI = 3.141592653589793;
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

// Emissive
@group(0) @binding(15)
var t_emissive: texture_2d<f32>;
@group(0) @binding(16)
var s_emissive: sampler;
@group(0) @binding(17)
var<uniform> m_emissive: vec4f;

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

const ARRAY_SIZE: u32 = 16u;
struct DirLight {
    @location(0) position: vec3f,
    @location(1) direction: vec3f,
    @location(2) intensity: f32,
    @location(3) shadow_map: i32,
    // @location(4) texture_map: i32,
    @location(4) coord_proj: mat4x4<f32>,
    @location(5) color: vec4f,
}
@group(3) @binding(0)
var<uniform> dirlights: array<DirLight, ARRAY_SIZE>;
@group(3) @binding(1)
var<uniform> dl_nums: u32; 
@group(3) @binding(1)
var t_dirlight: texture_depth_2d;
@group(3) @binding(2)
var s_dirlight: sampler;

// struct SpotLight {
//     @location(0) position: vec3f,
//     @location(1) direction: vec3f,
//     @location(2) intensity: f32,
//     @location(3) cast_shadow: u32,
//     @location(4) color: vec4f,
//     @location(5) coord_proj: mat4x4<f32>, 
// }
// @group(3) @binding(0)
// var<storage> pointlights: array<PointLight>;
// @group(3) @binding(1)
// var t_pointlight: texture_depth_cube_array;
// @group(3) @binding(2)
// var s_pointlight: sampler;


// @group(5) @binding(0)
// var<storage> spotlights: array<SpotLight>;
// @group(5) @binding(1)
// var t_spotlight: texture_depth_2d_array;
// @group(5) @binding(2)
// var s_spotlight: sampler;


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
    let normal = normalize(in.normal);
    let tangent = normalize(in.tangent.xyz);
    let bitangent = normalize(cross(normal, tangent)) * in.tangent.w;
    let pbr_normal = textureSample(t_normal, s_normal, in.texcoord).xyz - vec3(0.5);
    let tbn = mat3x3(tangent, bitangent, normal);

    let N = normalize(tbn * pbr_normal);
    let V = normalize(view.position - in.position);
    let albedo = textureSample(t_albedo, s_albedo, in.texcoord);
    let metallic = textureSample(t_metallic, s_metallic, in.texcoord).r;
    let roughness = textureSample(t_roughness, s_roughness, in.texcoord).r;
    let ao = textureSample(t_ao, s_ao, in.texcoord);
    let F0 = mix(vec3f(0.2), albedo.rgb, metallic);
    var l_out = vec3f(0.0);
    // Directional Light
    for (var it = 0u; it < dl_nums; it++) {
        // Cook-Torrance specular BRDF
        let dW = 1.0 / f32(dl_nums);
        let L = normalize(dirlights[it].position - in.position);
        let H = normalize(V + L);

        let D = normal_distribution(N, H, roughness);
        let G = geometry_smith(N, V, L, roughness);
        let F = fresnel_schlick(H, V, F0);
        let Ks = F;
        let Kd = (vec3f(1.0) - Ks) * (1.0 - metallic);

        let specular = D * F * G / (4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001);
        let radiance = dirlights[it].intensity * dirlights[it].color.rgb * dirlights[it].color.a;
        l_out += (Kd * albedo.rgb / PI + specular) * radiance * max(dot(N, L), 0.0);
    }
    let ambient = vec3f(0.02) * albedo.rgb * ao.rgb;
    var color = ambient + l_out;
    
    return vec4f(reinhard_tonemapping(color.xyz), 1.0);
}

// Trowbridge-Reitz GGX
fn normal_distribution(n: vec3f, h: vec3f, roughness: f32) -> f32 {
    let roughness_squared = pow(roughness, 2.0);
    let dot_nh = max(dot(n, h), 0.0);
    return roughness_squared / (PI * pow(pow(dot_nh, 2.0) * (roughness_squared - 1.0) + 1.0, 2.0));
}

// Smith's Schlick-GGX
fn geometry_smith(n: vec3f, v: vec3f, l: vec3f, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = r * r / 8.0;
    let n_v = max(dot(n, v), 0.0);
    let n_l = max(dot(n, l), 0.0);
    let ggx1 = n_v / (n_v * (1.0 - k) + k);
    let ggx2 = n_l / (n_l * (1.0 - k) + k);
    return ggx1 * ggx2;
}

// Fresnel-Schlick Approximation
fn fresnel_schlick(h: vec3f, v: vec3f, f: vec3f) -> vec3f {
    return f + vec3f(1.0 - f) * pow(1.0 - dot(h, v), 5.0);
}

fn reinhard_tonemapping(color: vec3f) -> vec3f {
    return pow(color / vec3(color + vec3f(1.0)), vec3(1.0 / 2.2));
}