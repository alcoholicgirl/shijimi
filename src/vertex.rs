use crate::bytecast;

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub texcoord: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x3,
        3 => Float32x4,
    ];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>().try_into().unwrap(),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(C)]
pub struct CanvasVertex {
    pub position: [f32; 3],
    pub texcoord: [f32; 2],
}

impl CanvasVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
    ];
    const QUAD: [Self; 4] = [
        Self {
            position: [-1.0, -1.0, 0.0],
            texcoord: [0.0, 1.0],
        },
        Self {
            position: [-1.0, 1.0, 0.0],
            texcoord: [0.0, 0.0],
        },
        Self {
            position: [1.0, -1.0, 0.0],
            texcoord: [1.0, 1.0],
        },
        Self {
            position: [1.0, 1.0, 0.0],
            texcoord: [1.0, 0.0],
        },
    ];
    const QUAD_INDEX: [u16; 6] = [0, 1, 2, 1, 2, 3];
    pub fn request_quad_buffer(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer) {
        let vertex = <wgpu::Device as wgpu::util::DeviceExt>::create_buffer_init(&device, &wgpu::util::BufferInitDescriptor {
            label: Some("QUAD Vertex Buffer"),
            contents: bytecast::cast_bytes(&Self::QUAD),
            usage: wgpu::BufferUsages::VERTEX
        });
        let index = <wgpu::Device as wgpu::util::DeviceExt>::create_buffer_init(&device, &wgpu::util::BufferInitDescriptor {
            label: Some("QUAD Index Buffer"),
            contents: bytecast::cast_bytes(&Self::QUAD_INDEX),
            usage: wgpu::BufferUsages::INDEX
        });
        (vertex, index)
    }
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>().try_into().unwrap(),
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}
