use crate::bytecast;
use wgpu::util::DeviceExt;

#[allow(unused)]
pub enum Texture {
    Offline {
        width: u32,
        height: u32,
        data: Vec<u8>,
        modulation: [f32; 4],
    },
    Online {
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        sampler: wgpu::Sampler,
        modulation: Option<(wgpu::Buffer, [f32; 4])>,
    },
}

impl Texture {
    pub fn submit(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if let Texture::Offline {
            width,
            height,
            data,
            modulation,
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
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Modulation buffer"),
                contents: bytecast::cast_bytes(modulation),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            *self = Texture::Online {
                texture: tex_buf,
                view: tex_view,
                sampler: tex_sampler,
                modulation: Some((buffer, *modulation)),
            }
        }
    }

    // RGBA8
    pub fn from_rgba8_bytes(data: &[u8], width: u32, height: u32) -> Self {
        Self::Offline {
            width,
            height,
            data: data.to_vec(),
            modulation: [1.0; 4],
        }
    }

    pub fn create_render_attachment(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Attachment"),
            size,
            dimension: wgpu::TextureDimension::D2,
            sample_count: 1,
            mip_level_count: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        Self::Online {
            texture,
            view,
            sampler,
            modulation: None,
        }
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn create_depth_buffer(device: &wgpu::Device, width: u32, height: u32) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Buffer"),
            size,
            dimension: wgpu::TextureDimension::D2,
            sample_count: 1,
            mip_level_count: 1,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Depth Buffer Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 200.0,
            ..Default::default()
        });
        Self::Online {
            texture,
            view,
            sampler,
            modulation: None,
        }
    }

    pub fn view(&self) -> Option<&wgpu::TextureView> {
        if let Texture::Online { view, .. } = self {
            Some(view)
        } else {
            None
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if let Texture::Online { texture, .. } = self {
            texture.destroy();
        }
    }
}
