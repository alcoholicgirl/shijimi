use crate::texture::Texture;

#[allow(unused)]
#[derive(Default)]
pub struct Material {
    pub albedo: Option<Texture>,
    pub normal: Option<Texture>,
    pub metallic: Option<Texture>,
    pub roughness: Option<Texture>,
    pub ao: Option<Texture>,
    pub emission: Option<Texture>,
    pub transmission: Option<Texture>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Material {
    pub fn submit(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        if self.albedo.is_none() {
            self.albedo = Some(Texture::from_rgba8_bytes([0x00u8; 4].as_slice(), 1, 1));
        }
        if self.normal.is_none() {
            self.normal = Some(Texture::from_rgba8_bytes(
                [0x80, 0x80, 0xFF, 0xFF].as_slice(),
                1,
                1,
            ));
        }
        if self.metallic.is_none() {
            self.metallic = Some(Texture::from_rgba8_bytes([0x00u8; 4].as_slice(), 1, 1));
        }
        if self.roughness.is_none() {
            self.roughness = Some(Texture::from_rgba8_bytes([0x00u8; 4].as_slice(), 1, 1));
        }
        if self.ao.is_none() {
            self.ao = Some(Texture::from_rgba8_bytes([0xFFu8; 4].as_slice(), 1, 1));
        }
        if self.emission.is_none() {
            self.emission = Some(Texture::from_rgba8_bytes([0x00u8; 4].as_slice(), 1, 1));
        }
        if self.transmission.is_none() {
            self.transmission = Some(Texture::from_rgba8_bytes([0x00u8; 4].as_slice(), 1, 1));
        }

        self.albedo.as_mut().unwrap().submit(device, queue);
        self.normal.as_mut().unwrap().submit(device, queue);
        self.metallic.as_mut().unwrap().submit(device, queue);
        self.roughness.as_mut().unwrap().submit(device, queue);
        self.ao.as_mut().unwrap().submit(device, queue);
        self.emission.as_mut().unwrap().submit(device, queue);
        self.transmission.as_mut().unwrap().submit(device, queue);

        let mut iota = 0;
        let mut iota = move || {
            iota += 1;
            iota - 1
        };
        let entries = [
            &self.albedo,
            &self.normal,
            &self.metallic,
            &self.roughness,
            &self.ao,
            &self.emission,
            &self.transmission,
        ]
        .iter()
        .map(|texture| {
            [wgpu::BindGroupEntry {
                    binding: iota(),
                    resource: wgpu::BindingResource::TextureView(
                        match texture.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: iota(),
                    resource: wgpu::BindingResource::Sampler(
                        match texture.as_ref().unwrap() {
                            Texture::Online { sampler, .. } => sampler,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: iota(),
                    resource: match texture.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },
            ]
        }).flatten().collect::<Vec<_>>();
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material.bind_group"),
            layout: &texture_bind_group_layout,
            entries: entries.as_slice()
        });
        self.bind_group = Some(bind_group);
    }
}
