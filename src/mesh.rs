use gfx::{Bundle, Resources};
use gfx::pso::PipelineData;

pub struct Mesh<R: Resources, D: PipelineData<R>>(Bundle<R, D>);

impl<R: Resources, D: PipelineData<R>> Mesh<R, D> {
    pub fn new() -> Self { unimplemented!() }

    pub fn from_builder(builder: MeshBuilder) -> Result<Self, BuildError> {
        unimplemented!();
    }
}

#[derive(Clone, Default, Eq, PartialEq)]
pub struct MeshBuilder {
    mesh_filename: &'static str,
    img_filename: Option<&'static str>
}

impl MeshBuilder {
    pub fn with_mesh(self, filename: &'static str) -> Self {
        MeshBuilder { mesh_filename: filename, ..self }
    }

    pub fn with_texture(self, filename: &'static str) -> Self {
        MeshBuilder { img_filename: Some(filename), ..self }   
    }

    pub fn build<R: Resources, D: PipelineData<R>>(self) -> Result<Mesh<R, D>, BuildError> {
        Mesh::from_builder(self)
    }
}

pub enum BuildError { }
