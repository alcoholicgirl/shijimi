use wgpu::util::DeviceExt;

use crate::{
    bytecast,
    camera::{Camera, CameraUniform},
    light::LightServer,
    mesh::{MeshNode, MeshServer},
    texture::Texture,
    vertex::Vertex,
};

pub struct RenderServer<'a> {
    surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    screen_texture: Texture,
    // Pipeline & Bind Group Layouts
    pub pbr_pipeline: PBRPipeline,
    pub shadow_mapping_pipeline: ShadowMappingPipeline,

    post_processing_bind_group_layout: wgpu::BindGroupLayout,
    post_processing_sampler: wgpu::Sampler,
    post_processing_pipeline: wgpu::RenderPipeline,

    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a winit::window::Window,
}

impl<'a> RenderServer<'a> {
    pub async fn new(window: &'a winit::window::Window) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        log::info!("config.format={:?}", config.format);
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                        | wgpu::Features::BUFFER_BINDING_ARRAY
                        | wgpu::Features::MULTIVIEW,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        // Main Render Pipeline
        let pbr_pipeline = PBRPipeline::new(&device, &config);
        // Shadow Mapping Pipeline
        let shadow_mapping_pipeline = ShadowMappingPipeline::new(&device);
        // Post Process
        let screen_buffer =
            Texture::create_render_attachment(&device, config.width, config.height, config.format);
        let post_processing_bind_group_layout =
            device.create_bind_group_layout(&Self::POST_PROCESS_BIND_GROUP_LAYOUT_DESC);

        let post_processing_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shader/post_process.wgsl"));
        let post_processing_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("state.post_processing_pipeline_layout"),
                bind_group_layouts: &[&post_processing_bind_group_layout],
                push_constant_ranges: &[],
            });
        let post_processing_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            ..Default::default()
        });
        let post_processing_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("state.post_procesing_pipeline"),
                layout: Some(&post_processing_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &post_processing_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_processing_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pbr_pipeline,
            shadow_mapping_pipeline,
            screen_texture: screen_buffer,
            post_processing_bind_group_layout,
            post_processing_sampler,
            post_processing_pipeline,
            size,
            window,
        })
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);

            // Create new depth buffer corresponding to screen size
            self.pbr_pipeline.resize(size, &self.device);

            let screen_texture = Texture::create_render_attachment(
                &self.device,
                size.width,
                size.height,
                self.config.format,
            );
            let old_screen_texture = std::mem::replace(&mut self.screen_texture, screen_texture);
            drop(old_screen_texture);
        }
    }

    fn try_render(
        &self,
        mesh_server: &MeshServer,
        light_server: &LightServer,
    ) -> Result<(), wgpu::SurfaceError> {
        let surface_output = self.surface.get_current_texture()?;
        let surface_view = surface_output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("state.render_encoder"),
            });

        // Lighting and Shadow Mapping
        light_server.update_uniform(&self.queue);
        // Render pass
        self.pbr_pipeline.render(
            &self.queue,
            &self.screen_texture,
            light_server,
            mesh_server,
            &mut encoder,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("state.post_processing_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let post_processing_bind_group =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("state.post_processing_bind_group"),
                    layout: &self.post_processing_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &self.screen_texture.view().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.post_processing_sampler),
                        },
                    ],
                });

            render_pass.set_pipeline(&self.post_processing_pipeline);
            render_pass.set_bind_group(0, Some(&post_processing_bind_group), &[]);
            render_pass.draw(0..6, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        surface_output.present();
        Ok(())
    }

    pub fn render(&mut self, mesh_server: &MeshServer, light_server: &LightServer) {
        match self.try_render(mesh_server, light_server) {
            Ok(_) => (),
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.resize(self.size);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out Of Memory");
                std::process::exit(1);
            }
            Err(wgpu::SurfaceError::Timeout) => {
                log::warn!("Timeout");
            }
            Err(err) => {
                log::warn!("{:?}", err);
            }
        }
    }

    pub fn submit_mesh_node(&self, node: &mut MeshNode) {
        node.submit(
            &self.device,
            &self.queue,
            &self.pbr_pipeline.texture_bind_group_layout,
            &self.pbr_pipeline.model_bind_group_layout,
        );
    }

    pub fn update(&self) {}
    pub const POST_PROCESS_BIND_GROUP_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'a> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("state.post_processing_bind_group_layout_desc"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        };
}

pub struct PBRPipeline {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    view_bind_group_layout: wgpu::BindGroupLayout,
    model_bind_group_layout: wgpu::BindGroupLayout,

    aspect_ratio: f32,
    active_camera: Camera,
    active_camera_buffer: wgpu::Buffer,
    active_camera_bind_group: wgpu::BindGroup,

    depth_buffer: Texture,
}

impl PBRPipeline {
    fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("../shader/pbr_main.wgsl"));
        let texture_bind_group_layout =
            device.create_bind_group_layout(&Self::TEXTURE_BIND_GROUP_LAYOUT_DESC);
        let view_bind_group_layout =
            device.create_bind_group_layout(&Self::VIEW_BIND_GROUP_LAYOUT_DESC);
        let model_bind_group_layout =
            device.create_bind_group_layout(&Self::MODEL_BIND_GROUP_LAYOUT_DESC);

        let light_bind_group_layout =
            device.create_bind_group_layout(&LightServer::DIR_LIGHT_LAYOUT);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("state.render_pipeline_layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &view_bind_group_layout,
                    &model_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("state.render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let depth_buffer = Texture::create_depth_buffer(&device, config.width, config.height);
        let active_camera = Camera::default();
        let aspect_ratio = 1.0;
        let active_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("main.camera.buffer"),
            contents: bytecast::cast_bytes(&active_camera.uniform(aspect_ratio)),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
        let active_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("main.camera.bind_group"),
            layout: &view_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: active_camera_buffer.as_entire_binding(),
            }],
        });
        Self {
            pipeline,
            texture_bind_group_layout,
            view_bind_group_layout,
            model_bind_group_layout,
            depth_buffer,

            aspect_ratio,
            active_camera,
            active_camera_buffer,
            active_camera_bind_group,
        }
    }

    fn render(
        &self,
        queue: &wgpu::Queue,
        screen_texture: &Texture,
        light_server: &LightServer,
        mesh_server: &MeshServer,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let camera_uniform = self.active_camera.uniform(self.aspect_ratio);
        queue
            .write_buffer_with(
                &self.active_camera_buffer,
                0,
                std::num::NonZero::new(std::mem::size_of::<CameraUniform>() as u64).unwrap(),
            )
            .unwrap()
            .clone_from_slice(bytecast::cast_bytes(&camera_uniform));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("state.render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_texture.view().unwrap(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: match &self.depth_buffer {
                    Texture::Online { view, .. } => view,
                    _ => unreachable!(),
                },
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(3, &light_server.dl_bind_group, &[]);
        mesh_server.request_draw(&self.active_camera_bind_group, &mut render_pass);
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device) {
        let depth_buffer = Texture::create_depth_buffer(device, size.width, size.height);
        let old_buffer = std::mem::replace(&mut self.depth_buffer, depth_buffer);
        drop(old_buffer); // just in case
        self.aspect_ratio = size.width as f32 / size.height as f32;
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.active_camera
    }

    const TEXTURE_BIND_GROUP_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'_> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("state.texture_bind_group_layout"),
            entries: &[
                // Albedo
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Normal
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Metallic
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Roughness
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 10,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 11,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 12,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 13,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 14,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 15,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 16,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 17,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 18,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 19,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 20,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        };

    const VIEW_BIND_GROUP_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'_> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("state.view_bind_group_layout_desc"),
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

    const MODEL_BIND_GROUP_LAYOUT_DESC: wgpu::BindGroupLayoutDescriptor<'_> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("state.model_bind_group_layout_desc"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        };
}

struct ShadowMappingPipeline {
    pipeline: wgpu::RenderPipeline,
    view_bind_group_layout: wgpu::BindGroupLayout,
    model_bind_group_layout: wgpu::BindGroupLayout,
}

impl ShadowMappingPipeline {
    fn new(device: &wgpu::Device) -> Self {
        let sm_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shader/shadow_mapping.wgsl"));
        let view_bind_group_layout =
            device.create_bind_group_layout(&PBRPipeline::VIEW_BIND_GROUP_LAYOUT_DESC);
        let model_bind_group_layout =
            device.create_bind_group_layout(&PBRPipeline::MODEL_BIND_GROUP_LAYOUT_DESC);

        let sm_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("state.shadow_map_pipeline_layout"),
            bind_group_layouts: &[&view_bind_group_layout, &model_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("state.shadow_map_pipeline"),
            layout: Some(&sm_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sm_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &sm_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            view_bind_group_layout,
            model_bind_group_layout,
        }
    }
}
