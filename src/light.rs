use wgpu::util::DeviceExt;

use crate::{bytecast, mesh::MeshServer};

pub enum LightSource {
    Directional {
        position: glam::Vec3,
        rotation: glam::Quat,
        intensity: f32,
        cast_shadow: bool,
        // texture: Option<Texture>,
        color: [f32; 4],
        ortho_window: (f32, f32),
        depth: f32,
    },
}

pub struct LightClient {
    entry: usize,
    source: LightSource,
}

pub struct LightServer {
    lights: Vec<LightClient>,

    // Directional Lights
    // Shadow Mapping Phase
    dl_sm_bind_group: wgpu::BindGroup,
    dl_sm_buffer: wgpu::Buffer,

    // Rasterization Phase
    dl_layout: wgpu::BindGroupLayout,
    dl_buffer: wgpu::Buffer,
    dl_texture: wgpu::Texture,
    dl_view: wgpu::TextureView,
    dl_sampler: wgpu::Sampler,
}

impl LightServer {
    const SHADOW_MAP_SIZE: u32 = 1024;
    const ARRAY_SIZE: u32 = 8;

    pub fn new(device: &wgpu::Device) -> Self {
        let lights = vec![];
        let dl_layout = device.create_bind_group_layout(&Self::DIR_LIGHT_LAYOUT);
        let dl_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dir Light Buffer"),
            size: Self::ARRAY_SIZE as u64 * std::mem::size_of::<DirLightUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dl_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Dir Light Shadow Map"),
            size: wgpu::Extent3d {
                width: Self::SHADOW_MAP_SIZE,
                height: Self::SHADOW_MAP_SIZE,
                depth_or_array_layers: Self::ARRAY_SIZE,
            },
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            format: wgpu::TextureFormat::Depth32Float,
            sample_count: 1,
            mip_level_count: 1,
            view_formats: &[],
        });
        let dl_view = dl_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let dl_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let dl_sm_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dir Light View Buffer"),
            contents: bytecast::cast_bytes(&[[0f32; 4]; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let dl_proj_bind_group_layout = device.create_bind_group_layout(&Self::VIEW_BIND_LAYOUT);
        let dl_sm_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Dir Light View Bind Group"),
            layout: &dl_proj_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: dl_sm_buffer.as_entire_binding(),
            }],
        });
        Self {
            lights,
            dl_sm_buffer,
            dl_sm_bind_group,
            dl_layout,
            dl_buffer,
            dl_texture,
            dl_view,
            dl_sampler,
        }
    }

    pub fn request_shadow_map(
        &self,
        mesh_server: &MeshServer,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        shadow_mapping_pipeline: &wgpu::RenderPipeline,
    ) {
        for light in self.lights.iter() {
            if let LightSource::Directional { .. } = light.source {
                let view = match light.source {
                    LightSource::Directional { .. } => {
                        self.dl_texture.create_view(&wgpu::TextureViewDescriptor {
                            label: Some("Shadow Mapping View"),
                            format: Some(wgpu::TextureFormat::Depth32Float),
                            dimension: Some(wgpu::TextureViewDimension::D2Array),
                            array_layer_count: Some(Self::ARRAY_SIZE),
                            ..Default::default()
                        })
                    }
                };
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Shadow Mapping Pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                });
                render_pass.set_pipeline(shadow_mapping_pipeline);
                render_pass.set_bind_group(0, &self.dl_sm_bind_group, &[]);
                let view_proj = light.source.view_proj().to_cols_array_2d();
                let uniform = bytecast::cast_bytes(&view_proj);
                queue.write_buffer(&self.dl_sm_buffer, 0, uniform);
                mesh_server.request_draw(&self.dl_sm_bind_group, &mut render_pass);
            }
        }
    }

    pub fn update_uniform(&self, queue: &wgpu::Queue) {
        let mut d_entry = 0usize;
        for light in &self.lights {
            match &light.source {
                LightSource::Directional { .. } => {
                    let uniform = DirLightUniform::from(&light.source, d_entry);
                    let data = bytecast::cast_bytes(&uniform);
                    queue.write_buffer(
                        &self.dl_buffer,
                        (d_entry * std::mem::size_of::<DirLightUniform>()) as u64,
                        data,
                    );
                    d_entry += 1;
                }
            }
        }
    }

    /// Try to spawn a light from given light source
    /// Returns the entry of the spawned if possible
    pub fn request_light(&mut self, source: LightSource) -> Option<usize> {
        let used_entries = self
            .lights
            .iter()
            .filter(|x| std::mem::discriminant(&x.source) == std::mem::discriminant(&source))
            .map(|x| x.entry)
            .collect::<Vec<_>>();
        for entry in 0..Self::ARRAY_SIZE as usize {
            if !used_entries.contains(&entry) {
                let client = LightClient {
                    source,
                    entry
                };
                self.lights.push(client);
                return Some(entry);
            }
        }
        None
    }

    // Bind group layout in the main shader
    const DIR_LIGHT_LAYOUT: wgpu::BindGroupLayoutDescriptor<'_> = wgpu::BindGroupLayoutDescriptor {
        label: Some("Dir Light Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: Some(std::num::NonZero::new(LightServer::ARRAY_SIZE).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: Some(std::num::NonZero::new(LightServer::ARRAY_SIZE).unwrap()),
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
        ],
    };

    const VIEW_BIND_LAYOUT: wgpu::BindGroupLayoutDescriptor<'_> = wgpu::BindGroupLayoutDescriptor {
        label: Some("Shadow Mapping View Bind Group Layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    };
}

impl LightSource {
    /// Get the projection matrix
    fn view_proj(&self) -> glam::Mat4 {
        match self {
            Self::Directional {
                position,
                rotation,
                ortho_window,
                depth,
                ..
            } => {
                let direction = <glam::Vec4 as glam::Vec4Swizzles>::xyz(
                    glam::Mat4::from_quat(*rotation) * glam::Vec4::Z,
                );
                let look_at = glam::Mat4::look_to_lh(*position, direction, glam::Vec3::Y);
                let (x, y) = *ortho_window;
                let z = *depth;
                let ortho = glam::Mat4::orthographic_lh(-x, x, -y, y, 0.0, z);
                ortho * look_at
            }
        }
    }
}

#[repr(C, align(8))]
pub struct DirLightUniform {
    position: [f32; 3],
    direction: [f32; 3],
    intensity: f32,
    shadow_map: i32,
    coord_proj: [[f32; 4]; 4],
    color: [f32; 4],
}

impl DirLightUniform {
    const DEFAULT_DIRECTION: glam::Vec4 = glam::Vec4::Z;

    /// This function ensures that source is directional light
    fn from(source: &LightSource, entry: usize) -> Self {
        match source {
            LightSource::Directional {
                position,
                rotation,
                intensity,
                cast_shadow,
                color,
                ..
            } => {
                let position = position.to_array();
                let direction = <glam::Vec4 as glam::Vec4Swizzles>::xyz(
                    glam::Mat4::from_quat(*rotation) * Self::DEFAULT_DIRECTION,
                )
                .to_array();
                DirLightUniform {
                    position,
                    direction,
                    intensity: *intensity,
                    shadow_map: if *cast_shadow {
                        entry.try_into().unwrap()
                    } else {
                        !0
                    },
                    coord_proj: source.view_proj().to_cols_array_2d(),
                    color: *color,
                }
            }
            _ => unreachable!(),
        }
    }
}
