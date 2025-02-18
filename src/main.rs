use light::LightServer;
use mesh::MeshServer;
use spatial::Spatial;
use wgpu::util::DeviceExt;

mod bytecast;
mod light;
mod material;
mod mesh;
mod spatial;
mod state;
mod texture;
mod vertex;
mod camera;
use bytecast::*;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut state = async_std::task::block_on(async { state::State::new(&window).await.unwrap() });
    let mut surface_configured = false;

    // Scene building
    let mut mesh_server = MeshServer::new();
    let mut light_server = LightServer::new(&state.device);
    let dlight = light_server.request_light(
        light::LightSource::Directional {
            position: glam::vec3(0.0, 1.0, 0.0),
            rotation: glam::Quat::from_rotation_y(15f32.to_radians()),
            intensity: 1.0f32,
            cast_shadow: true,
            color: [1.0; 4],
            ortho_window: (10.0, 10.0),
            depth: 10.0f32,
        },
    );

    let mut node = mesh::MeshNode::load_from_path("assets/model/hiroi/scene.gltf")?;
    let mut camera = camera::Camera::default();
    let aspect_ratio = state.size.width as f32 / state.size.height as f32;
    let camera_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("main.camera.buffer"),
            contents: cast_bytes(&camera.uniform(aspect_ratio)),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });
    let view_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("main.camera.bind_group"),
        layout: &state.view_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });
    node.submit(
        &state.device,
        &state.queue,
        &state.texture_bind_group_layout,
        &state.model_bind_group_layout,
    );

    let mushroom_node = mesh_server.add_node(node);
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
                        state.window.request_redraw();
                        if !surface_configured {
                            return;
                        }
                        {
                            let node = mesh_server.get_mut(mushroom_node).unwrap();
                            let aspect_ratio = state.size.width as f32 / state.size.height as f32;
                            frame += 1;
                            let ftime = frame as f32 * 0.004;
                            camera.position =
                                glam::Mat3::from_rotation_y(ftime * 0.2) * glam::Vec3::Z * 2.0;
                            camera.look_at(glam::Vec3::ZERO);
                            let camera_uniform = camera.uniform(aspect_ratio);
                            state.queue.write_buffer(
                                &camera_buffer,
                                0,
                                bytecast::cast_bytes(&camera_uniform),
                            );

                            node.mesh.apply_model(
                                &state.queue,
                                glam::Mat4::from_translation(glam::Vec3 {
                                    x: 0.0,
                                    y: -0.8,
                                    z: 0.0,
                                }) * glam::Mat4::from_scale(glam::Vec3::ONE * 0.5),
                            );
                            state.render(&mesh_server, &light_server, &view_bind_group);
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
