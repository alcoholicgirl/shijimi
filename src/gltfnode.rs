use crate::vertex::Vertex;

pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

#[derive(Default)]
pub struct GltfNode {
    name: Option<String>,
    children: Vec<GltfNode>,
    mesh: Option<Mesh>,
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

fn construct_node(node: &gltf::Node, buffers: &Vec<gltf::buffer::Data>, depth: u32) -> GltfNode {
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
            println!();
            intent();
            if primitive.mode() != gltf::mesh::Mode::Triangles {
                panic!("This Gltf model not triangulated");
            }
            let reader = primitive.reader(|buf| Some(&buffers[buf.index()]));

            use gltf::mesh::util::*;

            // Indices
            let indices = if let Some(ReadIndices::U16(gltf::accessor::Iter::Standard(iter))) =
                reader.read_indices()
            {
                iter.collect::<Vec<_>>()
            } else {
                panic!("No indices");
            };

            // Positions
            let positions = if let Some(ReadPositions::Standard(iter)) = reader.read_positions() {
                iter.collect::<Vec<_>>()
            } else {
                panic!("No positions");
            };

            // Texcoords
            let texcoords = if let Some(ReadTexCoords::F32(iter)) = reader.read_tex_coords(0) {
                iter.collect::<Vec<_>>()
            } else {
                panic!("No texcoords");
            };

            // Normals
            let normals = if let Some(iter) = reader.read_normals() {
                iter.collect::<Vec<_>>()
            } else {
                panic!("No normals");
            };

            // Tangents
            let tangents = if let Some(iter) = reader.read_tangents() {
                iter.collect::<Vec<_>>()
            } else {
                panic!("No tangents");
            };

            assert_eq!(positions.len(), texcoords.len());
            assert_eq!(positions.len(), normals.len());
            assert_eq!(positions.len(), tangents.len());

            use iter_tools::dependency::itertools::izip;
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
        let child = construct_node(&child, &buffers, depth + 1);
        gltfnode.push(child);
    }
    gltfnode
}

pub fn load_gltf_from_slice(data: &[u8]) -> anyhow::Result<GltfNode> {
    let (gltf, buffers, images) = gltf::import_slice(data)?;
    let mut root_node = GltfNode::default();
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            root_node.push(construct_node(&node, &buffers, 0));
        }
    }
    Ok(root_node)
}
