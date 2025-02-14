pub trait Spatial {
    fn set_position(&mut self, position: glam::Vec3) {}
    fn set_rotation(&mut self, rotation: glam::Quat) {}
    fn set_scale(&mut self, scale: glam::Vec3) {}

    fn get_position(&self) -> glam::Vec3 {
        glam::Vec3::ZERO
    }
    fn get_rotation(&self) -> glam::Quat {
        glam::Quat::IDENTITY
    }
    fn get_scale(&self) -> glam::Vec3 {
        glam::Vec3::ONE
    }
}
