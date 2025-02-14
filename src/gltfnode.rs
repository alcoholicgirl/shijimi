use crate::{bytecast, material::Material, texture::Texture, vertex::Vertex};
use gltf::mesh::util::*;
use iter_tools::dependency::itertools::izip;
use wgpu::util::DeviceExt;

#[allow(unused)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    buffers: Option<(wgpu::Buffer, wgpu::Buffer)>,
}

pub struct GltfNode {
    id: usize,
    name: Option<String>,
    children: Vec<GltfNode>,
    mesh: Option<Mesh>,
    material: Material,
    position: glam::Vec3,
    rotation: glam::Quat,
    scale: glam::Vec3,
    uniform_buffer: Option<wgpu::Buffer>,
    model_bind_group: Option<wgpu::BindGroup>,
}

pub struct RenderResource<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub texture_bind_group: &'a wgpu::BindGroup,
    pub model_bind_group: &'a wgpu::BindGroup,
}

fn build(
    node: &gltf::Node,
    buffers: &Vec<gltf::buffer::Data>,
    images: &Vec<gltf::image::Data>,
    depth: u32,
) -> GltfNode {
    let intent = || {
        for _ in 0..depth {
            print!(" ");
        }
    };
    intent();
    print!(
        "Node #{}: {}",
        node.index(),
        if let Some(name) = node.name() {
            name
        } else {
            "(null)"
        }
    );

    let (position, rotation, scale) = node.transform().decomposed();
    let mut gltfnode = GltfNode {
        position: glam::Vec3::from(position),
        rotation: glam::Quat::from_array(rotation),
        scale: glam::Vec3::from(scale),
        ..Default::default()
    };
    if let Some(name) = node.name() {
        gltfnode.name = Some(name.to_string());
    }
    gltfnode.id = node.index();

    if let Some(mesh) = node.mesh() {
        print!("(contains mesh)");
        for primitive in mesh.primitives() {
            // Material
            let mut material = Material::default();
            let rgba8_loader = |image: &gltf::image::Data| match image.format {
                gltf::image::Format::R8G8B8A8 => image.pixels.clone(),
                gltf::image::Format::R8G8B8 => {
                    let alpha = 0xffu8;
                    image
                        .pixels
                        .clone()
                        .chunks_exact(3)
                        .into_iter()
                        .map(|rgb| [rgb[0], rgb[1], rgb[2], alpha].into_iter())
                        .flatten()
                        .collect::<Vec<_>>()
                }
                _ => panic!("Unsupported format"),
            };

            // Albedo
            if let Some(tex) = primitive
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                let albedo = &images.get(tex.texture().index());
                if let Some(albedo) = albedo {
                    material.albedo = Some(Texture::Offline {
                        width: albedo.width,
                        height: albedo.height,
                        data: rgba8_loader(albedo),
                        modulation: primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_factor(),
                    });
                }
            }

            // Normal
            if let Some(normal) = primitive.material().normal_texture() {
                let id = normal.texture().index();
                let texture = &images.get(id);
                if let Some(texture) = texture {
                    material.normal = Some(Texture::Offline {
                        width: texture.width,
                        height: texture.height,
                        data: rgba8_loader(texture),
                        modulation: [1.0; 4],
                    });
                }
            }

            // Metallic
            if let Some(tex) = primitive
                .material()
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
            {
                let texture = &images.get(tex.texture().index());
                if let Some(texture) = texture {
                    let data = rgba8_loader(texture);
                    let metallic = data
                        .clone()
                        .chunks_exact(4)
                        .map(|rgba| {
                            let f = rgba[2];
                            [f; 4].into_iter()
                        })
                        .flatten()
                        .collect();
                    let roughness = data
                        .clone()
                        .chunks_exact(4)
                        .map(|rgba| {
                            let f = rgba[1];
                            [f; 4].into_iter()
                        })
                        .flatten()
                        .collect();

                    material.metallic = Some(Texture::Offline {
                        width: texture.width,
                        height: texture.height,
                        data: metallic,
                        modulation: [primitive
                            .material()
                            .pbr_metallic_roughness()
                            .metallic_factor(); 4],
                    });

                    material.roughness = Some(Texture::Offline {
                        width: texture.width,
                        height: texture.height,
                        data: roughness,
                        modulation: [primitive
                            .material()
                            .pbr_metallic_roughness()
                            .roughness_factor(); 4],
                    });
                }
            }

            // AO
            if let Some(tex) = primitive.material().occlusion_texture() {
                let texture = &images.get(tex.texture().index());
                if let Some(texture) = texture {
                    material.ao = Some(Texture::Offline {
                        width: texture.width,
                        height: texture.height,
                        data: rgba8_loader(texture),
                        modulation: [1.0; 4],
                    });
                }
            }

            gltfnode.material = material;

            // Geometry
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                panic!("Gltf model not triangulated");
            }
            let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));
            // Positions
            let positions = if let Some(ReadPositions::Standard(iter)) = reader.read_positions() {
                iter.collect::<Vec<_>>()
            } else {
                panic!("No positions");
            };
            // Indices
            let indices: Vec<u32> = match reader.read_indices() {
                Some(ReadIndices::U8(iter)) => iter.map(|x| x as u32).collect(),
                Some(ReadIndices::U16(iter)) => iter.map(|x| x as u32).collect(),
                Some(ReadIndices::U32(iter)) => iter.collect(),
                None => {
                    if let Ok(size) = <usize as TryInto<u32>>::try_into(positions.len()) {
                        (0..size).collect()
                    } else {
                        panic!("Failed to construct indices.");
                    }
                }
            };

            // Texcoords
            let texcoords = if let Some(ReadTexCoords::F32(iter)) = reader.read_tex_coords(0) {
                iter.collect::<Vec<_>>()
            } else {
                log::warn!("No texcoords. Texcoords are set to (0, 0) by default.");
                positions.iter().map(|_| [0f32, 0f32]).collect()
            };

            // Normals
            let normals = if let Some(iter) = reader.read_normals() {
                iter.collect::<Vec<_>>()
            } else {
                // Construct normals
                if indices.len() % 3 != 0 {
                    panic!("Gltf model not triangulated: {} indices", indices.len());
                }
                log::info!("No normals specified. Constructing normals.");
                let normals = indices
                    .iter()
                    .map(|x| positions[*x as usize])
                    .collect::<Vec<_>>()
                    .chunks(3)
                    .map(|p| {
                        // let p : Vec<_>= p.collect();
                        let (p1, p2, p3) = (p[0], p[1], p[2]);
                        let (p1, p2, p3) = (
                            glam::Vec3::from(p1),
                            glam::Vec3::from(p2),
                            glam::Vec3::from(p3),
                        );
                        let normal = (p3 - p2).cross(p1 - p2).normalize().to_array();
                        [normal, normal, normal].into_iter()
                    })
                    .flatten()
                    .collect::<Vec<[f32; 3]>>();
                let mut normals_processed: Vec<glam::Vec3> =
                    (0..positions.len()).map(|_| Default::default()).collect();
                for (index, normal) in indices.iter().zip(normals) {
                    normals_processed[*index as usize] += glam::Vec3::from(normal);
                }
                let normals = normals_processed
                    .iter()
                    .map(|n| n.normalize().to_array())
                    .collect();
                normals
            };

            // Tangents
            let tangents = if let Some(iter) = reader.read_tangents() {
                iter.collect::<Vec<_>>()
            } else {
                // Construct tangents from normals and UVs
                if indices.len() % 3 != 0 {
                    panic!("Gltf model not triangulated: {} indices", indices.len());
                }
                let positions_flattened = indices
                    .iter()
                    .map(|x| positions[*x as usize])
                    .collect::<Vec<_>>();

                let texcoords_flattened = indices
                    .iter()
                    .map(|x| texcoords[*x as usize])
                    .collect::<Vec<_>>();

                let pos_uv = izip!(
                    positions_flattened.chunks_exact(3),
                    texcoords_flattened.chunks_exact(3),
                );
                let tangents = pos_uv
                    .map(|(position, texcoord)| {
                        let (p1, p2, p3) = (
                            glam::Vec3::from(position[0]),
                            glam::Vec3::from(position[1]),
                            glam::Vec3::from(position[2]),
                        );
                        let (t1, t2, t3) = (
                            glam::Vec2::from(texcoord[0]),
                            glam::Vec2::from(texcoord[1]),
                            glam::Vec2::from(texcoord[2]),
                        );
                        let duv1 = t1 - t2;
                        let duv2 = t3 - t2;
                        let e1 = p1 - p2;
                        let e2 = p3 - p2;
                        let v1 = glam::vec2(duv2.y, -duv1.y);
                        let v2 = glam::vec2(-duv2.x, duv1.x);
                        let tangent = [
                            v1.x * e1.x - v2.y * e2.x,
                            v1.x * e1.y - v2.y * e2.y,
                            v1.x * e1.z - v2.y * e2.z,
                            // 1.0,
                        ];
                        [tangent, tangent, tangent].into_iter()
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                let mut tangents_processed: Vec<glam::Vec3> =
                    (0..positions.len()).map(|_| Default::default()).collect();

                for (index, tangent) in indices.iter().zip(tangents) {
                    tangents_processed[*index as usize] += glam::Vec3::from(tangent);
                }
                let tangents = tangents_processed
                    .iter()
                    .map(|t| {
                        let t = t.normalize();
                        glam::vec4(t.x, t.y, t.z, 1.0).to_array()
                    })
                    .collect();
                tangents
            };

            assert_eq!(positions.len(), texcoords.len());
            assert_eq!(positions.len(), normals.len());
            assert_eq!(positions.len(), tangents.len());

            let vertex_iter = izip!(positions, texcoords, normals, tangents);
            let vertices = vertex_iter
                .map(|(position, texcoord, normal, tangent)| Vertex {
                    position,
                    texcoord,
                    normal,
                    tangent,
                })
                .collect::<Vec<_>>();
            let mesh = Mesh {
                vertices,
                indices,
                buffers: None,
            };
            gltfnode.mesh = Some(mesh);
        }
    }

    println!();
    for child in node.children() {
        let child = build(&child, &buffers, images, depth + 1);
        gltfnode.push(child);
    }
    gltfnode
}

impl Mesh {
    pub fn submit(&mut self, device: &wgpu::Device) -> anyhow::Result<()> {
        if self.buffers.is_some() {
            return Err(anyhow::anyhow!("Mesh buffers already submitted"));
        }
        let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytecast::cast_bytes_vec(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytecast::cast_bytes_vec(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.buffers = Some((vb, ib));
        Ok(())
    }
}

impl GltfNode {
    // Initialize GltfNode
    pub fn submit(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        model_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        log::info!("Node #{}: requesting texture bind group", self.id);
        if self.material.bind_group.is_none() {
            self.material
                .submit(device, queue, texture_bind_group_layout);
        }

        log::info!("Node #{}: requesting mesh buffers", self.id);
        if let Some(mesh) = &mut self.mesh {
            if mesh.buffers.is_none() {
                let _ = mesh.submit(device);
            }
        }

        log::info!(
            "Node #{}: requesting uniform buffer and uniform bind group",
            self.id
        );
        let model = self.model_matrix().to_cols_array_2d();
        self.uniform_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Uniform Buffer"),
                contents: bytecast::cast_bytes(&model),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }),
        );

        self.model_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Uniform Bind Group"),
            layout: &model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.uniform_buffer.as_ref().unwrap().as_entire_binding(),
            }],
        }));

        for child in &mut self.children {
            child.submit(
                device,
                queue,
                texture_bind_group_layout,
                model_bind_group_layout,
            );
        }
    }

    pub fn load_from_slice(data: &[u8]) -> anyhow::Result<Self> {
        let (gltf, buffers, images) = gltf::import_slice(data)?;
        let mut root_node = GltfNode::default();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                root_node.push(build(&node, &buffers, &images, 0));
            }
        }
        Ok(root_node)
    }

    pub fn load_from_path(path: &'static str) -> anyhow::Result<Self> {
        let (gltf, buffers, images) = gltf::import(path)?;
        let mut root_node = GltfNode::default();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                root_node.push(build(&node, &buffers, &images, 0));
            }
        }
        Ok(root_node)
    }

    pub fn push(&mut self, child: GltfNode) {
        self.children.push(child);
    }

    pub fn model_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_translation(self.position)
            * glam::Mat4::from_quat(self.rotation)
            * glam::Mat4::from_scale(self.scale)
    }

    pub fn set_position(&mut self, position: glam::Vec3) {
        self.position = position;
    }

    pub fn set_rotation(&mut self, rotation: glam::Quat) {
        self.rotation = rotation;
    }

    pub fn set_scale(&mut self, scale: glam::Vec3) {
        self.scale = scale;
    }

    pub fn apply_model(&self, queue: &wgpu::Queue, model: glam::Mat4) {
        // Update model matrix uniform
        let t_model = model * self.model_matrix();
        let uniform_data = t_model.to_cols_array_2d();
        let uniform = bytecast::cast_bytes(&uniform_data);
        if let Some(buffer) = &self.uniform_buffer {
            queue.write_buffer(buffer, 0, uniform);
        }

        for child in &self.children {
            child.apply_model(queue, t_model);
        }
    }

    pub fn request_resources(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Vec<RenderResource> {
        let mut resources = vec![];
        if let Some(mesh) = &self.mesh {
            if let Some((vb, ib)) = &mesh.buffers {
                if let (Some(texture_bind_group), Some(model_bind_group)) =
                    (&self.material.bind_group, &self.model_bind_group)
                {
                    resources.push(RenderResource {
                        vertex_buffer: vb,
                        index_buffer: ib,
                        model_bind_group,
                        texture_bind_group,
                    });
                } else {
                    log::warn!(
                        "Node #{}: buffers or(and) bind groups not initialized, skipping.",
                        self.id
                    );
                }
            } else {
                log::warn!("Node #{}: mesh not initialized, skipping.", self.id);
            }
        }

        for child in &self.children {
            for resource in child.request_resources(device, queue) {
                resources.push(resource);
            }
        }
        resources
    }
}

impl Default for GltfNode {
    fn default() -> Self {
        Self {
            id: 0,
            name: None,
            children: vec![],
            mesh: None,
            material: Material::default(),
            uniform_buffer: None,
            model_bind_group: None,
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
        }
    }
}
