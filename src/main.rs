use wgpu::util::DeviceExt;

mod bytecast;
mod camera;
mod gltfnode;
mod state;
mod vertex;
use bytecast::*;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new()?;
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut state = async_std::task::block_on(async { state::State::new(&window).await.unwrap() });
    let mut surface_configured = false;
    let mut node = gltfnode::GltfNode::load_from_path("assets/model/9mm/scene.gltf")?;

    let mut camera = camera::Camera::default();
    let aspect_ratio = state.size.width as f32 / state.size.height as f32;
    let camera_buffer = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: to_bytes(&camera.uniform(aspect_ratio)),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

    let camera_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera Bind Group"),
        layout: &state.camera_bind_group_layout,
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
                            let aspect_ratio = state.size.width as f32 / state.size.height as f32;
                            frame += 1;
                            let ftime = frame as f32 * 0.002;
                            camera.position =
                                glam::Mat3::from_rotation_y(ftime * 0.5) * glam::Vec3::Z * 2.0;
                            camera.look_at(glam::Vec3::ZERO);
                            let camera_uniform = camera.uniform(aspect_ratio);
                            state.queue.write_buffer(
                                &camera_buffer,
                                0,
                                bytecast::to_bytes(&camera_uniform),
                            );

                            let resources = node.prepare_draw(
                                &state.device,
                                &state.queue,
                                glam::Mat4::IDENTITY,
                            );
                            for resource in resources.iter() {
                                state.render(
                                    resource.vertex_buffer,
                                    resource.index_buffer,
                                    resource.texture_bind_group,
                                    resource.model_bind_group,
                                    &camera_bind_group,
                                );
                            }

                            // state.render(vertex_buffer, index_buffer, texture_bind_group, &camera_bind_group, model_bind_group);
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
