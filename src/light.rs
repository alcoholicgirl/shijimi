use glam::Vec4Swizzles;
use wgpu::util::DeviceExt;

use crate::{bytecast, spatial::Spatial};

pub const SHADOW_MAPPING_SIZE: (u32, u32) = (1024, 1024);
pub enum LightSource {
    PointLight { position: glam::Vec3 },
    DirLight { direction: glam::Quat, size: f32 },
}

pub struct ShadowMappingBuffer {
    pub sm_texture: wgpu::Texture,
    pub sm_view: wgpu::TextureView,
    pub sm_sampler: wgpu::Sampler,
}

pub struct LightNode {
    pub source: LightSource,
    pub view_bind_group: wgpu::BindGroup,
    pub view_buffer: wgpu::Buffer,
    pub sm_buffer: Option<ShadowMappingBuffer>,
}

pub struct LightResource<'a> {
    pub light: &'a LightNode,
}

#[repr(C, align(16))]
#[allow(unused)]
pub struct LightUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 3],
}

impl LightNode {
    pub fn new(
        source: LightSource,
        cast_shadow: bool,
        device: &wgpu::Device,
        view_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let sm_buffer = if cast_shadow {
            let sm_sample_size = wgpu::Extent3d {
                width: SHADOW_MAPPING_SIZE.0,
                height: SHADOW_MAPPING_SIZE.1,
                depth_or_array_layers: match source {
                    LightSource::PointLight { .. } => 6,
                    LightSource::DirLight { .. } => 1,
                },
            };
            let sm_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Shadow Mapping Depth Buffer"),
                dimension: wgpu::TextureDimension::D2,
                sample_count: 1,
                size: sm_sample_size,
                mip_level_count: 1,
                format: wgpu::TextureFormat::Depth32Float,
                view_formats: &[],
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            });
            let sm_view = sm_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("Shadow Mapping View"),
                dimension: Some(match source {
                    LightSource::PointLight { .. } => wgpu::TextureViewDimension::Cube,
                    LightSource::DirLight { .. } => wgpu::TextureViewDimension::D2,
                }),
                ..Default::default()
            });
            let sm_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Shadow Mapping Sampler"),
                ..Default::default()
            });
            Some(ShadowMappingBuffer {
                sm_texture,
                sm_view,
                sm_sampler,
            })
        } else {
            None
        };
        let view_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light View Uniform Buffer"),
            contents: bytecast::cast_bytes(&[0.0; 80]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Light View Bind Group"),
            layout: view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(view_buffer.as_entire_buffer_binding()),
            }],
        });
        Self {
            source,
            sm_buffer,
            view_buffer,
            view_bind_group,
        }
    }

    pub fn uniform(&self) -> LightUniform {
        LightUniform {
            view_pos: self.get_position().to_array(),
            view_proj: match self.source {
                LightSource::DirLight { direction, size } => {
                    let dir = (glam::Mat4::from_quat(direction) * glam::Vec4::NEG_Y).xyz();
                    let ortho = glam::Mat4::orthographic_lh(-size, size, -size, size, 0.1, 100.0);
                    let look_at = glam::Mat4::look_to_lh(self.get_position(), dir, glam::Vec3::Y);
                    ortho * look_at
                }
                LightSource::PointLight { .. } => glam::Mat4::IDENTITY,
            }
            .to_cols_array_2d(),
        }
    }
}

impl Spatial for LightNode {
    fn set_position(&mut self, _position: glam::Vec3) {
        match &mut self.source {
            LightSource::PointLight { position } => {
                *position = _position;
            }
            _ => (),
        }
    }

    fn set_rotation(&mut self, rotation: glam::Quat) {
        match &mut self.source {
            LightSource::DirLight { direction, .. } => {
                *direction = rotation;
            }
            _ => (),
        }
    }

    fn get_position(&self) -> glam::Vec3 {
        match &self.source {
            LightSource::PointLight { position } => *position,
            _ => Default::default(),
        }
    }

    fn get_rotation(&self) -> glam::Quat {
        match &self.source {
            LightSource::DirLight { direction, .. } => *direction,
            _ => Default::default(),
        }
    }
}
