use na::{Matrix4};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraMatrices {
    pub view: Matrix4<f32>,
    pub projection: Matrix4<f32>,
}

impl CameraMatrices {
	pub fn new(view: Matrix4<f32>, projection: Matrix4<f32>) -> Self {
		CameraMatrices { view, projection, }
	}
}

pub trait Camera {
    fn matrices(&self) -> CameraMatrices;
}
