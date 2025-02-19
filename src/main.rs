mod bytecast;
mod camera;
mod light;
mod material;
mod mesh;
mod renderserver;
mod spatial;
mod texture;
mod vertex;
use camera::Camera;
use light::LightServer;
use mesh::MeshServer;
use spatial::{Spatial, SpatialExt};
use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let window = winit::window::WindowBuilder::new().build(&event_loop)?;
    let mut server = async_std::task::block_on(async {
        renderserver::RenderServer::new(&window).await.unwrap()
    });
    let mut surface_configured = false;

    // Scene building
    let mut mesh_server = MeshServer::new();
    let mut light_server = LightServer::new(&server.device);
    let l1 = light_server.request_light(light::LightSource::Directional {
        position: glam::vec3(0.0, 4.0, 0.0),
        rotation: glam::Quat::from_rotation_y(15f32)
            * glam::Quat::from_rotation_x(-45f32.to_radians()),
        intensity: 40.0f32,
        cast_shadow: true,
        color: [0.9, 0.7, 0.5, 1.0],
        ortho_window: (10.0, 10.0),
        depth: 10.0f32,
    })?;

    let l2 = light_server.request_light(light::LightSource::Directional {
        position: glam::vec3(0.0, 4.0, 0.0),
        rotation: glam::Quat::from_rotation_y(-15f32)
            * glam::Quat::from_rotation_x(-45f32.to_radians()),
        intensity: 80.0f32,
        cast_shadow: true,
        color: [0.4, 0.9, 0.6, 1.0],
        ortho_window: (10.0, 10.0),
        depth: 10.0f32,
    })?;
    let l3 = light_server.request_light(light::LightSource::Directional {
        position: glam::vec3(0.0, 4.0, 0.0),
        rotation: glam::Quat::from_rotation_y(-45f32)
            * glam::Quat::from_rotation_x(75f32.to_radians()),
        intensity: 50.0f32,
        cast_shadow: true,
        color: [0.5, 0.5, 0.8, 1.0],
        ortho_window: (10.0, 10.0),
        depth: 10.0f32,
    })?;

    let mut node = mesh::MeshNode::load_from_path("assets/model/lasergun/scene.gltf")?;
    server.submit_mesh_node(&mut node);
    let node_entry = mesh_server.add_node(node);
    let mut frame = 0;
    event_loop.run(move |ev: Event<()>, control_flow| match ev {
        Event::DeviceEvent { .. } => {}
        Event::WindowEvent {
            window_id,
            ref event,
        } => {
            if window_id == server.window.id() {
                match event {
                    WindowEvent::Resized(size) => {
                        server.resize(*size);
                        surface_configured = true;
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        let camera = server.pbr_pipeline.camera();
                        let rotation = camera.get_rotation();
                        let right = rotation.mul_vec3(Camera::RIGHT);
                        let right = right - right.project_onto(Camera::UP);
                        let front = rotation.mul_vec3(Camera::FRONT);
                        let front = front - front.project_onto(Camera::UP);
                        let up = Camera::UP;
                        let speed = 0.2;
                        let aspeed = 0.2;

                        if let PhysicalKey::Code(key) = event.physical_key {
                            match key {
                                KeyCode::KeyW => {
                                    camera.translate(front * speed);
                                }
                                KeyCode::KeyA => {
                                    camera.translate(-right * speed);
                                }
                                KeyCode::KeyS => {
                                    camera.translate(-front * speed);
                                }
                                KeyCode::KeyD => {
                                    camera.translate(right * speed);
                                }
                                KeyCode::KeyQ => {
                                    camera.translate(up * speed);
                                }
                                KeyCode::KeyE => {
                                    camera.translate(-up * speed);
                                }
                                KeyCode::ArrowLeft => {
                                    camera.rotate(glam::Quat::from_rotation_y(aspeed));
                                }
                                KeyCode::ArrowRight => {
                                    camera.rotate(glam::Quat::from_rotation_y(-aspeed));
                                }
                                KeyCode::ArrowUp =>{
                                    camera.rotate(glam::Quat::from_axis_angle(right, aspeed));
                                }
                                KeyCode::ArrowDown=>{
                                    camera.rotate(glam::Quat::from_axis_angle(-right, aspeed));
                                }
                                KeyCode::KeyR => {
                                    camera.fov *= 0.9;
                                }
                                KeyCode::KeyF => {
                                    camera.fov /= 0.9;
                                }
                                _ => (),
                            }
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        server.update();
                        server.window.request_redraw();
                        if !surface_configured {
                            return;
                        }
                        {
                            frame += 1;
                            let ftime = frame as f32 * 0.004;
                            let camera = server.pbr_pipeline.camera();
                            {
                                let node = mesh_server.get_mut(node_entry).unwrap();
                                node.mesh.update_uniform(
                                    &server.queue,
                                    glam::Mat4::from_scale(glam::Vec3::ONE * 20f32)
                                        * glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.24)),
                                    camera.get_view_matrix(),
                                );
                            }
                            server.render(&mesh_server, &light_server);
                        }
                    }
                    WindowEvent::CloseRequested => {
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
