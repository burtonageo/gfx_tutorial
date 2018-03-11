use na::{Matrix4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraMatrices {
    pub view: Matrix4<f32>,
    pub projection: Matrix4<f32>,
}

pub trait Camera {
    fn matrices(&self) -> CameraMatrices;
}
