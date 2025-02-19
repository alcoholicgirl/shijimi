use crate::spatial::{Spatial, SpatialExt};

#[derive(Debug)]
pub struct Camera {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub fov: f32,
}

#[repr(C)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    view_pos: [f32; 4],
}

impl Camera {
    pub const FRONT: glam::Vec3 = glam::Vec3::NEG_Z;
    pub const RIGHT: glam::Vec3 = glam::Vec3::X;
    pub const UP: glam::Vec3 = glam::Vec3::Y;
    pub fn look_at(&mut self, target: glam::Vec3) {
        self.rotation =
            glam::Quat::from_rotation_arc(Self::FRONT, (target - self.position).normalize());
    }
    pub fn get_view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_to_rh(
            self.position,
            self.rotation.mul_vec3(Self::FRONT),
            Self::UP,
        )
    }
    pub fn uniform(&self, aspect_ratio: f32) -> CameraUniform {
        let view = glam::Mat4::look_to_rh(
            self.position,
            self.rotation.mul_vec3(Self::FRONT),
            Self::UP
        );
        let proj = glam::Mat4::perspective_rh(self.fov, aspect_ratio, 0.1, 100.0);
        let view_proj = (proj * view).to_cols_array_2d();
        let view_pos = glam::Vec4::from((self.position, 1.0)).to_array();
        CameraUniform {
            view_proj,
            view_pos,
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: glam::vec3(0.0, 0.0, 1.0),
            rotation: glam::Quat::IDENTITY,
            fov: 45f32.to_radians(),
        }
    }
}

impl Spatial for Camera {
    fn get_position(&self) -> glam::Vec3 {
        self.position
    }
    fn get_rotation(&self) -> glam::Quat {
        self.rotation
    }
    fn set_position(&mut self, position: glam::Vec3) {
        self.position = position
    }
    fn set_rotation(&mut self, rotation: glam::Quat) {
        self.rotation = rotation
    }
}

impl SpatialExt for Camera {}
