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

pub trait SpatialExt: Spatial {
    fn translate(&mut self, translation: glam::Vec3) {
        self.set_position(self.get_position() + translation);
    }
    fn rotate(&mut self, rotation: glam::Quat) {
        self.set_rotation(rotation * self.get_rotation());
    }
    fn get_model_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_translation(self.get_position())
        * glam::Mat4::from_quat(self.get_rotation())
        * glam::Mat4::from_scale(self.get_scale())
    }
}