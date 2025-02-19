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
    @location(1) position: vec4f,
}
@group(1) @binding(0)
var<uniform> view: ViewUniform;

// Model
struct ModelUniform {
    @location(0) model: mat4x4<f32>,
    @location(1) normal_matrix: mat4x4<f32>,
}
@group(2) @binding(0)
var<uniform> model: ModelUniform;

// Lights

const ARRAY_SIZE: u32 = 8u;
struct DirLight {
    @location(0) position: vec3f,
    @location(1) intensity: f32,
    @location(2) direction: vec3f,
    @location(3) shadow_map: i32,
    @location(4) coord_proj: mat4x4<f32>,
    @location(5) color: vec4f,
}
struct DirLights {
    @location(0) num: u32,
    @location(1) lights: array<DirLight, ARRAY_SIZE>
}
@group(3) @binding(0)
var<uniform> dir_lights: DirLights;

struct VertexInput {
    @location(0) position: vec3f,
    @location(1) texcoord: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec3f, 
    @location(4) bitangent: vec3f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) worldpos: vec3f,
    @location(1) texcoord: vec2f,
    @location(2) normal: vec3f,
    @location(3) tangent: vec3f,
    @location(4) bitangent: vec3f,
};

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let worldpos = model.model * vec4(in.position, 1.0);
    out.clip_position = view.projection * worldpos;
    out.texcoord = in.texcoord;
    out.worldpos = worldpos.xyz;
    let normal_matrix = model.normal_matrix;
    out.normal = normalize((normal_matrix * vec4f(in.normal, 0.0)).xyz);
    out.tangent = normalize((normal_matrix * vec4f(in.tangent, 0.0)).xyz);
    out.bitangent = normalize((normal_matrix * vec4f(in.bitangent, 0.0)).xyz);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // Normal Mapping
    let normal = normalize(in.normal);
    let tangent = normalize(in.tangent);
    let bitangent = normalize(in.bitangent);
    let pbr_normal = textureSample(t_normal, s_normal, in.texcoord).xyz * 2.0 - vec3(1.0);
    let tbn = mat3x3(tangent, bitangent, normal);

    let N = normalize(tbn * pbr_normal);
    let V = normalize(view.position.xyz - in.worldpos);
    let albedo = pow(textureSample(t_albedo, s_albedo, in.texcoord).rgb, vec3f(2.2));
    let metallic = textureSample(t_metallic, s_metallic, in.texcoord).r;
    let roughness = textureSample(t_roughness, s_roughness, in.texcoord).r;
    let ao = textureSample(t_ao, s_ao, in.texcoord).r;
    let F0 = mix(vec3f(0.005), albedo, metallic);

    var l_out = vec3f(0.0);
    // Directional Light
    for (var it = 0u; it < dir_lights.num; it++) {
        // Cook-Torrance specular BRDF
        let light = dir_lights.lights[it];
        let L = normalize(-light.direction);
        let H = normalize(V + L);

        let D = normal_distribution(N, H, roughness);
        let G = geometry_smith(N, V, L, roughness);
        let F = fresnel_schlick(H, V, F0);
        let specular = D * F * G / (4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.001);
        let Ks = F;
        let Kd = (vec3f(1.0) - Ks) * (1.0 - metallic);

        let radiance = light.intensity * light.color.rgb * light.color.a;
        l_out += (Kd * albedo / PI + specular) * radiance * max(dot(N, L), 0.0);
    }
    let ambient = vec3f(0.12, 0.12, 0.2) * albedo.rgb * ao;
    let color = ambient + l_out;
    return vec4f(reinhard_tonemapping(color), 1.0);
}   

// Trowbridge-Reitz GGX
fn normal_distribution(n: vec3f, h: vec3f, roughness: f32) -> f32 {
    let r = roughness * roughness;
    let r2 = r * r;
    let dot_nh = max(dot(n, h), 0.0);
    let dot_nh_2 = dot_nh * dot_nh;
    let dm = dot_nh_2 * (r2 - 1.0) + 1.0;
    return r2 / (PI * dm * dm);
}

fn geometry_schlick_ggx(dot_nv: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = r * r * 0.125;
    let n = dot_nv;
    let d = dot_nv * (1.0 - k) + k;
    return n / d;
}

// Smith's Schlick-GGX
fn geometry_smith(n: vec3f, v: vec3f, l: vec3f, roughness: f32) -> f32 {
    let n_v = max(dot(n, v), 0.0);
    let n_l = max(dot(n, l), 0.0);

    let ggx1 = geometry_schlick_ggx(n_v, roughness);
    let ggx2 = geometry_schlick_ggx(n_l, roughness);

    return ggx1 * ggx2;
}

// Fresnel-Schlick Approximation
fn fresnel_schlick(h: vec3f, v: vec3f, f: vec3f) -> vec3f {
    return f + (vec3f(1.0) - f) * pow(clamp(1.0 - dot(h, v), 0.0, 1.0), 5.0);
}

fn reinhard_tonemapping(color: vec3f) -> vec3f {
    return pow(color / vec3(color + vec3f(1.0)), vec3(1.0 / 2.2));
}