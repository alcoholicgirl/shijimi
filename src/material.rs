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

        self.albedo.as_mut().unwrap().submit(device, queue);
        self.normal.as_mut().unwrap().submit(device, queue);
        self.metallic.as_mut().unwrap().submit(device, queue);
        self.roughness.as_mut().unwrap().submit(device, queue);
        self.ao.as_mut().unwrap().submit(device, queue);
        self.emission.as_mut().unwrap().submit(device, queue);

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("material.bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                // Albedo
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        match &self.albedo.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(
                        match &self.albedo.as_ref().unwrap() {
                            Texture::Online { sampler, .. } => sampler,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: match &self.albedo.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },
                // Normal
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        match &self.normal.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(
                        match &self.normal.as_ref().unwrap() {
                            Texture::Online { sampler, .. } => sampler,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: match &self.normal.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },
                // Metallic
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(
                        match &self.metallic.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: wgpu::BindingResource::Sampler(
                        match &self.metallic.as_ref().unwrap() {
                            Texture::Online { sampler, .. } => sampler,
                            _ => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: match &self.metallic.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },
                // Roughness
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: wgpu::BindingResource::TextureView(
                        match &self.roughness.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 10,
                    resource: wgpu::BindingResource::Sampler(
                        match &self.roughness.as_ref().unwrap() {
                            Texture::Online { sampler, .. } => sampler,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 11,
                    resource: match &self.roughness.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },
                // AO
                wgpu::BindGroupEntry {
                    binding: 12,
                    resource: wgpu::BindingResource::TextureView(
                        match &self.ao.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 13,
                    resource: wgpu::BindingResource::Sampler(match &self.ao.as_ref().unwrap() {
                        Texture::Online { sampler, .. } => sampler,
                        Texture::Offline { .. } => unreachable!(),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 14,
                    resource: match self.ao.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },

                // Emissive
                wgpu::BindGroupEntry {
                    binding: 15,
                    resource: wgpu::BindingResource::TextureView(
                        match &self.emission.as_ref().unwrap() {
                            Texture::Online { view, .. } => view,
                            Texture::Offline { .. } => unreachable!(),
                        },
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 16,
                    resource: wgpu::BindingResource::Sampler(match &self.ao.as_ref().unwrap() {
                        Texture::Online { sampler, .. } => sampler,
                        Texture::Offline { .. } => unreachable!(),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 17,
                    resource: match self.emission.as_ref().unwrap() {
                        Texture::Online {
                            modulation: Some((buffer, _)),
                            ..
                        } => buffer.as_entire_binding(),
                        _ => unreachable!(),
                    },
                },
            ],
        });
        self.bind_group = Some(bind_group);
    }
}
