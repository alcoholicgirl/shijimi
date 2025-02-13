use wgpu::util::DeviceExt;

mod camera;
mod gltfnode;
mod vertex;

mod state;

fn main() -> anyhow::Result<()> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut state = async_std::task::block_on(async { state::State::new(&window).await.unwrap() });
    let mut surface_configured = false;
    // let node = gltfnode::load_gltf_from_path("assets/model/9mm/scene.gltf")?;

    let vertices = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: to_bytes(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

    let indices = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: to_bytes(&INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

    let mut camera = camera::Camera::default();
    let aspect_ratio = state.size.width as f32 / state.size.height as f32;
    let camera_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: to_bytes(&camera.uniform(aspect_ratio)),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
    let (tex_view, tex_sampler) =
        state.request_texture_rgba8(include_bytes!("../assets/image/60.png"))?;
    let texture_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Texture Bind Group"),
        layout: &state.texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&tex_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&tex_sampler),
            },
        ],
    });
    let camera_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera Bind Group"),
        layout: &state.camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let model = glam::Mat4::IDENTITY.to_cols_array_2d();
    let model_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Buffer"),
            contents: to_bytes(&model),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
    let model_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Model Bind Group"),
        layout: &state.model_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: model_buffer.as_entire_binding(),
        }],
    });

    let mut frame = 0;
    event_loop.run(move |ev, control_flow| match ev {
        winit::event::Event::WindowEvent {
            window_id,
            ref event,
        } => {
            if window_id == state.window.id() {
                match event {
                    winit::event::WindowEvent::Resized(size) => {
                        state.resize(*size);
                        surface_configured = true;
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        state.update();
                        let aspect_ratio = state.size.width as f32 / state.size.height as f32;
                        frame += 1;
                        let ftime = frame as f32 * 0.002;

                        let rotation = ftime.sin();
                        let position = glam::vec3(0.0, (ftime * 2.0).sin() * 0.2 + 0.4, 0.0);
                        let scale = glam::vec3(1.0, (ftime * 2.0 + 1.0).sin() * 0.4 + 1.0, 1.0);
                        let model = glam::Mat4::from_scale(scale);
                        let model = glam::Mat4::from_translation(position) * model;
                        let model = glam::Mat4::from_rotation_z(rotation * 0.05) * model;
                        let model_uniform = model.to_cols_array_2d();
                        camera.position =
                            glam::Mat3::from_rotation_y(ftime * 0.5) * glam::Vec3::Z * 2.0;
                        camera.look_at(glam::Vec3::ZERO);
                        let camera_uniform = camera.uniform(aspect_ratio);

                        // write uniforms
                        state
                            .queue
                            .write_buffer(&camera_buffer, 0, to_bytes(&camera_uniform));
                        state
                            .queue
                            .write_buffer(&model_buffer, 0, to_bytes(&model_uniform));
                        state.window.request_redraw();
                        if !surface_configured {
                            return;
                        }
                        match state.render(
                            &vertices,
                            &indices,
                            &texture_bind_group,
                            &camera_bind_group,
                            &model_bind_group,
                        ) {
                            Ok(_) => (),
                            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                                state.resize(state.size);
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => {
                                log::error!("Out Of Memory");
                                control_flow.exit();
                            }
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Timeout");
                            }
                            Err(err) => {
                                log::warn!("{:?}", err);
                            }
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        control_flow.exit();
                    }
                    _ => (),
                }
            }
        }
        _ => (),
    })?;
    Ok(())
}

pub fn to_bytes<T: Sized>(src: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts((src as *const T).cast(), std::mem::size_of::<T>()) }
}

use vertex::Vertex;
const VERTICES: [Vertex; 4] = [
    Vertex {
        position: [-0.5, -0.5, 0.0],
        texcoord: [0.0, 1.0],
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5, 0.0],
        texcoord: [0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        texcoord: [1.0, 1.0],
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        texcoord: [1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        tangent: [1.0, 0.0, 0.0, 1.0],
    },
];

const INDICES: [u32; 6] = [0, 2, 1, 1, 2, 3];
