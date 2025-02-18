#[derive(Debug)]
pub struct Camera {
    pub position: glam::Vec3,
    pub direction: glam::Vec3,
    pub fov: f32,
}

#[repr(C)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 4],
}

impl Camera {
    pub fn look_at(&mut self, target: glam::Vec3) {
        self.direction = target - self.position;
    }
    pub fn uniform(&self, aspect_ratio: f32) -> CameraUniform {
        let view = glam::Mat4::look_to_lh(self.position, self.direction, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_lh(self.fov, aspect_ratio, 0.1, 100.0);
        let view_proj = (proj * view).to_cols_array_2d();
        let view_pos = glam::Vec4::from((self.position, 1.0)).to_array();
        CameraUniform {
            view_proj,
            view_pos
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: glam::vec3(0.0, 0.0, 1.0),
            direction: glam::Vec3::NEG_Z,
            fov: 35f32.to_radians(),
        }
    }
}
