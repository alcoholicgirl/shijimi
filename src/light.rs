use glam::Vec4Swizzles;

use crate::{bytecast, spatial::Spatial};

// These constants MUST be consistent with the ones in shader
const MAX_POINT_LIGHTS: u32 = 32;
const MAX_DIR_LIGHTS: u32 = 32;
const MAX_SPOT_LIGHTS: u32 = 32;

// For each type of light source, shadow maps are stored within one large texture
// Corresponding shadow maps can be accessed with an given offset
const SHADOW_MAP_SIZE: u32 = 1024;

const POINT_LIGHT_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'_> =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Point Light Bind Group Layout Descriptor"),
        entries: &[
            // Point lights array
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: Some(std::num::NonZero::new(MAX_POINT_LIGHTS).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::CubeArray,
                    multisampled: false,
                },
                count: Some(std::num::NonZero::new(MAX_POINT_LIGHTS).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    };

const DIR_LIGHT_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'_> =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Dir Light Bind Group Layout Descriptor"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: Some(std::num::NonZero::new(MAX_DIR_LIGHTS).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: Some(std::num::NonZero::new(MAX_DIR_LIGHTS).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    };

const SPOT_LIGHT_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'_> =
    wgpu::BindGroupLayoutDescriptor {
        label: Some("Spot Light Bind Group Layout Descriptor"),
        entries: &[
            // Point lights array
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: Some(std::num::NonZero::new(MAX_SPOT_LIGHTS).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: Some(std::num::NonZero::new(MAX_SPOT_LIGHTS).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    };

pub struct LightManager {
    point_lights: Vec<PointLight>,
    // point_light_buffer: wgpu::Buffer,
    point_light_bind_group_layout: wgpu::BindGroupLayout,
    point_light_shadow_map: wgpu::Texture,
    point_light_shadow_map_view: wgpu::TextureView,
    point_light_shadow_map_sampler: wgpu::Sampler,

    dir_lights: Vec<DirLight>,
    // dir_light_buffer: wgpu::Buffer,
    dir_light_bind_group_layout: wgpu::BindGroupLayout,
    dir_light_shadow_map: wgpu::Texture,
    dir_light_shadow_map_view: wgpu::TextureView,
    dir_light_shadow_map_sampler: wgpu::Sampler,

    spot_lights: Vec<SpotLight>,
    // spot_light_buffer: wgpu::Buffer,
    spot_light_bind_group_layout: wgpu::BindGroupLayout,
    spot_light_shadow_map: wgpu::Texture,
    spot_light_shadow_map_view: wgpu::TextureView,
    spot_light_shadow_map_sampler: wgpu::Sampler,
}

pub struct LightHandle {}

pub struct PointLight {
    position: glam::Vec3,
    intensity: f32,
    color: [f32; 4],
    range: f32,
    shadow_map: Option<wgpu::Texture>,
}

pub struct DirLight {
    position: glam::Vec3,
    direction: glam::Quat,
    intensity: f32,
    color: [f32; 4],
    range: (f32, f32, f32),
    shadow_map: Option<wgpu::Texture>,
}

pub struct SpotLight {
    position: glam::Vec3,
    direction: glam::Quat,
    color: [f32; 4],
    radius: f32,
    range: f32,
    shadow_map: Option<wgpu::Texture>,
}

#[repr(C, align(16))]
struct PointLightUniform {
    position: [f32; 3],
    intensity: f32,
    cast_shadow: u32,
    color: [f32; 4],
}
#[repr(C, align(16))]
struct DirLightUniform {
    rotation: [f32; 3],
    intensity: f32,
    cast_shadow: u32,
    color: [f32; 4],
    /// Local Coordination Projection (inverse of view mat)
    coord_proj: [[f32; 4]; 4],
}

impl LightManager {
    const SHADOW_MAP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn new(device: &wgpu::Device) -> Self {
        let point_light_shadow_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Point Light Shadow Map"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 6,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::SHADOW_MAP_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let point_light_shadow_map_view =
            point_light_shadow_map.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Point Light Shadow Map View"),
                dimension: Some(wgpu::TextureViewDimension::Cube),
                ..Default::default()
            });
        let point_light_shadow_map_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let dir_light_shadow_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dir Light Shadow Map"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::SHADOW_MAP_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let dir_light_shadow_map_view =
            dir_light_shadow_map.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Dir Light Shadow Map View"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                ..Default::default()
            });

        let dir_light_shadow_map_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let spot_light_shadow_map = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Spot Light Shadow Map View"),
            dimension: wgpu::TextureDimension::D2,
            format: Self::SHADOW_MAP_FORMAT,
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
            view_formats: &[],
        });

        let spot_light_shadow_map_view =
            spot_light_shadow_map.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Spot Light Shadow Map View"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                ..Default::default()
            });

        let spot_light_shadow_map_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let point_light_bind_group_layout =
            device.create_bind_group_layout(&POINT_LIGHT_LAYOUT_DESC);
        let dir_light_bind_group_layout = device.create_bind_group_layout(&DIR_LIGHT_LAYOUT_DESC);
        let spot_light_bind_group_layout = device.create_bind_group_layout(&SPOT_LIGHT_LAYOUT_DESC);

        Self {
            point_light_shadow_map,
            point_light_shadow_map_view,
            point_light_shadow_map_sampler,
            dir_light_shadow_map,
            dir_light_shadow_map_view,
            dir_light_shadow_map_sampler,
            spot_light_shadow_map,
            spot_light_shadow_map_view,
            spot_light_shadow_map_sampler,
            point_light_bind_group_layout,
            dir_light_bind_group_layout,
            spot_light_bind_group_layout,
            point_lights: vec![],
            dir_lights: vec![],
            spot_lights: vec![],
        }
    }
    pub fn add_point_light(&mut self, light: PointLight) {
        self.point_lights.push(light);
    }
    pub fn add_dir_light(&mut self, light: DirLight) {
        self.dir_lights.push(light);
    }
    pub fn add_spot_light(&mut self, light: SpotLight) {
        self.spot_lights.push(light);
    }
}

impl PointLight {
    pub fn uniform(&self) -> PointLightUniform {
        todo!()
    }
    pub fn update(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        todo!()
    }
}

impl DirLight {
    const DEFAULT_DIRECTION: glam::Vec4 = glam::Vec4::NEG_Y;
    fn uniform(&self) -> DirLightUniform {
        DirLightUniform {
            rotation: (glam::Mat4::from_quat(self.direction) * Self::DEFAULT_DIRECTION)
                .xyz()
                .to_array(),
            intensity: self.intensity,
            cast_shadow: if self.shadow_map.is_some() { !0 } else { 0 },
            color: self.color,
            coord_proj: self.view_proj().inverse().to_cols_array_2d(),
        }
    }
    fn view_proj(&self) -> glam::Mat4 {
        let (x, y, z) = self.range;
        glam::Mat4::orthographic_lh(-x, x, -y, y, 0.0, z)
            * glam::Mat4::look_to_lh(
                self.position,
                (glam::Mat4::from_quat(self.direction) * Self::DEFAULT_DIRECTION).xyz(),
                glam::Vec3::Y,
            )
    }
    fn update_uniform(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer, entry: usize) {
        let uniform = self.uniform();
        let uniform_data = bytecast::cast_bytes(&uniform);
        queue.write_buffer(
            buffer,
            (std::mem::size_of::<DirLightUniform>() * entry) as u64,
            uniform_data,
        );
    }
}

impl Spatial for PointLight {
    fn get_position(&self) -> glam::Vec3 {
        self.position
    }
    fn set_position(&mut self, position: glam::Vec3) {
        self.position = position;
    }
}

impl Spatial for DirLight {
    fn get_rotation(&self) -> glam::Quat {
        self.direction
    }
    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.direction = rotation;
    }
}

impl Spatial for SpotLight {
    fn get_position(&self) -> glam::Vec3 {
        self.position
    }
    fn get_rotation(&self) -> glam::Quat {
        self.direction
    }
    fn set_position(&mut self, position: glam::Vec3) {
        self.position = position;
    }
    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.direction = rotation;
    }
}
