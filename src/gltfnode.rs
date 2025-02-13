use crate::vertex::Vertex;
use gltf::mesh::util::*;
use iter_tools::{dependency::itertools::izip, Itertools};

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

#[derive(Default)]
pub struct Material {
    albedo: Option<Texture>,
    normal: Option<Texture>,
    metallic_roughness: Option<Texture>,
    ao: Option<Texture>,
}

pub enum Texture {
    Offline {
        width: u32,
        height: u32,
        data: Vec<u8>,
        factor: [f32; 4],
    },
    Online {
        handle: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
        factor: [f32; 4],
    },
}

#[derive(Default)]
pub struct GltfNode {
    name: Option<String>,
    children: Vec<GltfNode>,
    mesh: Option<Mesh>,
}

fn construct_node(
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

    let mut gltfnode = GltfNode::default();
    if let Some(name) = node.name() {
        gltfnode.name = Some(name.to_string());
    }
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
                let albedo = &images[tex.texture().index()];
                material.albedo = Some(Texture::Offline {
                    width: albedo.width,
                    height: albedo.height,
                    data: rgba8_loader(albedo),
                    factor: primitive.material().pbr_metallic_roughness().base_color_factor()
                });
            }

            // Normal
            if let Some(normal) = primitive.material().normal_texture() {
                let id = normal.texture().index();
                let texture = &images[id];
                material.normal = Some(Texture::Offline {
                    width: texture.width,
                    height: texture.height,
                    data: rgba8_loader(texture),
                    factor: [1.0; 4]
                });
            }

            // Metallic-Roughness
            if let Some(tex) = primitive
                .material()
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
            {
                let texture = &images[tex.texture().index()];
                material.metallic_roughness = Some(Texture::Offline {
                    width: texture.width,
                    height: texture.height,
                    data: rgba8_loader(texture),
                    factor: [1.0; 4]
                });
            }

            // AO
            if let Some(tex) = primitive.material().occlusion_texture() {
                let texture = &images[tex.texture().index()];
                material.ao = Some(Texture::Offline {
                    width: texture.width,
                    height: texture.height,
                    data: rgba8_loader(texture),
                });
            }

            if primitive.mode() != gltf::mesh::Mode::Triangles {
                panic!("Gltf model not triangulated");
            }

            // Geometry
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
            let mesh = Mesh { vertices, indices };
            gltfnode.mesh = Some(mesh);
        }
    }

    println!();
    for child in node.children() {
        let child = construct_node(&child, &buffers, images, depth + 1);
        gltfnode.push(child);
    }
    gltfnode
}

pub fn load_gltf_from_slice(data: &[u8]) -> anyhow::Result<GltfNode> {
    let (gltf, buffers, images) = gltf::import_slice(data)?;
    let mut root_node = GltfNode::default();
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            root_node.push(construct_node(&node, &buffers, &images, 0));
        }
    }
    Ok(root_node)
}

pub fn load_gltf_from_path(path: &'static str) -> anyhow::Result<GltfNode> {
    let (gltf, buffers, images) = gltf::import(path)?;
    let mut root_node = GltfNode::default();
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            root_node.push(construct_node(&node, &buffers, &images, 0));
        }
    }
    Ok(root_node)
}

impl GltfNode {
    pub fn new() -> Self {
        Self {
            name: None,
            children: vec![],
            mesh: None,
        }
    }
    pub fn push(&mut self, child: GltfNode) {
        self.children.push(child);
    }
}

impl Texture {
    pub fn request_online(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Texture::Offline {
            width,
            height,
            data,
        } = self
        {
            let size = wgpu::Extent3d {
                width: *width,
                height: *height,
                depth_or_array_layers: 1,
            };
            let tex_buf = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size,
                dimension: wgpu::TextureDimension::D2,
                sample_count: 1,
                mip_level_count: 1,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            queue.write_texture(
                wgpu::TexelCopyTextureInfoBase {
                    texture: &tex_buf,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &data.as_slice(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4u32 * *width),
                    rows_per_image: Some(*height),
                },
                size,
            );
            let tex_view = tex_buf.create_view(&wgpu::TextureViewDescriptor::default());
            let tex_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            *self = Texture::Online {
                handle: tex_buf,
                view: tex_view,
                sampler: tex_sampler,
            }
        }
    }

    pub fn from_rgba8(rgba: [u8; 4]) -> Self {
        Self::Offline { width: 1, height: 1, data: rgba.to_vec() }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if let Texture::Online { handle, .. } = self {
            handle.destroy();
        }
    }
}

impl Material {
    pub fn generate_bind_group() {

    }
}