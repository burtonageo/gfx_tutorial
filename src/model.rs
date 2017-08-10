use {pipe, ColorFormat, DepthFormat, GLSL_VERT_SRC, GLSL_FRAG_SRC, MAX_LIGHTS, MSL_VERT_SRC,
     MSL_FRAG_SRC, ShaderLight, SharedLocals, VertLocals};
use gfx::{Bundle, CombinedError, CommandBuffer, Encoder, PipelineStateError, Primitive, Resources,
          UpdateError};
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx::state::Rasterizer;
use gfx::texture::{AaMode, Kind};
use image::{self, ImageError};
use load::{load_obj, LoadObjError};
use na::{Matrix4, Similarity3};
use std::error::Error;
use std::fmt;
use platform::{Backend, FactoryExt, WindowExt};
use util::get_assets_folder;

pub struct Model<R: Resources> {
    bundle: Bundle<R, pipe::Data<R>>,
    pub similarity: Similarity3<f32>,
}

impl<R: Resources> Model<R> {
    pub fn load<F: FactoryExt<R>>(
        factory: &mut F,
        backend: &Backend,
        rtv: RenderTargetView<R, ColorFormat>,
        dsv: DepthStencilView<R, DepthFormat>,
        model_name: &str,
        texture_name: &str,
    ) -> Result<Self, ModelLoadError> {
        let similarity = Similarity3::from_scaling(1.0);
        let bundle = {
            let program = if backend.is_gl() {
                factory.link_program(GLSL_VERT_SRC, GLSL_FRAG_SRC).unwrap()
            } else {
                factory.link_program(MSL_VERT_SRC, MSL_FRAG_SRC).unwrap()
            };

            let pso = factory.create_pipeline_from_program(
                &program,
                Primitive::TriangleList,
                Rasterizer::new_fill().with_cull_back(),
                pipe::new(),
            )?;

            let (_, srv) = {
                let mut img_path = get_assets_folder().unwrap().to_path_buf();
                img_path.push(texture_name);
                let img = image::open(img_path)?.to_rgba();
                let (iw, ih) = img.dimensions();
                let kind = Kind::D2(iw as u16, ih as u16, AaMode::Single);
                factory.create_texture_immutable_u8::<ColorFormat>(
                    kind,
                    &[&img],
                )?
            };

            let sampler = factory.create_sampler_linear();

            let (verts, inds) = load_obj(model_name)?;
            let (vertex_buffer, slice) =
                factory.create_vertex_buffer_with_slice(&verts[..], &inds[..]);
            let data = pipe::Data {
                vbuf: vertex_buffer,
                vert_locals: factory.create_constant_buffer(1),
                shared_locals: factory.create_constant_buffer(1),
                lights: factory.create_constant_buffer(MAX_LIGHTS),
                main_texture: (srv, sampler),
                out: rtv,
                main_depth: dsv,
            };

            Bundle::new(slice, pso, data)
        };
        Ok(Model { bundle, similarity })
    }

    #[inline]
    pub fn encode<C: CommandBuffer<R>>(&self, encoder: &mut Encoder<R, C>) {
        self.bundle.encode(encoder)
    }

    #[inline]
    pub fn update_matrices<C: CommandBuffer<R>>(
        &self,
        encoder: &mut Encoder<R, C>,
        view_matrix: &Matrix4<f32>,
        projection_matrix: &Matrix4<f32>,
    ) {
        let model_matrix = self.similarity.to_homogeneous();
        encoder.update_constant_buffer(
            &self.bundle.data.vert_locals,
            &VertLocals {
                model: *(model_matrix).as_ref(),
                view: *(view_matrix).as_ref(),
                projection: *(projection_matrix).as_ref(),
            },
        );
    }

    #[inline]
    pub fn update_lights<C: CommandBuffer<R>>(
        &self,
        encoder: &mut Encoder<R, C>,
        lights: &[ShaderLight],
    ) -> Result<(), UpdateError<usize>> {
        let num_lights = lights.len() as u32;
        assert!(num_lights < MAX_LIGHTS as u32);
        encoder.update_constant_buffer(
            &self.bundle.data.shared_locals,
            &SharedLocals { num_lights },
        );
        encoder.update_buffer(&self.bundle.data.lights, &lights, 0)
    }

    #[inline]
    pub fn update_views<W: WindowExt<R>>(&mut self, window: &W) {
        window.update_views(&mut self.bundle.data.out, &mut self.bundle.data.main_depth);
    }
}

#[derive(Debug)]
pub enum ModelLoadError {
    Obj(LoadObjError),
    Pso(PipelineStateError<String>),
    GfxTextureView(CombinedError),
    Image(ImageError),
}

impl fmt::Display for ModelLoadError {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        let desc = self.description();
        match *self {
            ModelLoadError::Obj(ref e) => write!(fmtr, "{}: {}", desc, e),
            ModelLoadError::Pso(ref e) => write!(fmtr, "{}: {}", desc, e),
            ModelLoadError::GfxTextureView(ref e) => write!(fmtr, "{}: {}", desc, e),
            ModelLoadError::Image(ref e) => write!(fmtr, "{}: {}", desc, e),
        }
    }
}

impl Error for ModelLoadError {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            ModelLoadError::Obj(_) => "The obj file could not be loaded",
            ModelLoadError::Pso(_) => "There was an error creating the pso",
            ModelLoadError::GfxTextureView(_) => {
                "An error occured while loading the texture on the gpu"
            }
            ModelLoadError::Image(_) => {
                "An error occurred while loading the texture image from disk"
            }
        }
    }

    #[inline]
    fn cause(&self) -> Option<&Error> {
        match *self {
            ModelLoadError::Obj(ref e) => Some(e),
            ModelLoadError::Pso(ref e) => Some(e),
            ModelLoadError::GfxTextureView(ref e) => Some(e),
            ModelLoadError::Image(ref e) => Some(e),
        }
    }
}

impl From<LoadObjError> for ModelLoadError {
    #[inline]
    fn from(e: LoadObjError) -> Self {
        ModelLoadError::Obj(e)
    }
}

impl From<PipelineStateError<String>> for ModelLoadError {
    #[inline]
    fn from(e: PipelineStateError<String>) -> Self {
        ModelLoadError::Pso(e)
    }
}

impl<'a> From<PipelineStateError<&'a str>> for ModelLoadError {
    #[inline]
    fn from(e: PipelineStateError<&'a str>) -> Self {
        ModelLoadError::Pso(e.into())
    }
}

impl From<CombinedError> for ModelLoadError {
    #[inline]
    fn from(e: CombinedError) -> Self {
        ModelLoadError::GfxTextureView(e)
    }
}

impl From<ImageError> for ModelLoadError {
    #[inline]
    fn from(e: ImageError) -> Self {
        ModelLoadError::Image(e)
    }
}
