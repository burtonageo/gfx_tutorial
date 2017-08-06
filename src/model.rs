use {pipe, ColorFormat, DepthFormat, GLSL_VERT_SRC, GLSL_FRAG_SRC, MAX_LIGHTS,
     MSL_VERT_SRC, MSL_FRAG_SRC, ShaderLight, SharedLocals, VertLocals};
use gfx::{Bundle, CommandBuffer, Encoder, Primitive, Resources};
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx::state::Rasterizer;
use gfx::texture::{AaMode, Kind};
use image;
use load::{load_obj, LoadObjError};
use na::{Matrix4, Similarity3};
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
    ) -> Result<Self, LoadObjError> {
        let program = if backend.is_gl() {
            factory.link_program(GLSL_VERT_SRC, GLSL_FRAG_SRC).unwrap()
        } else {
            factory.link_program(MSL_VERT_SRC, MSL_FRAG_SRC).unwrap()
        };

        let pso = factory
            .create_pipeline_from_program(
                &program,
                Primitive::TriangleList,
                Rasterizer::new_fill().with_cull_back(),
                pipe::new(),
            )
            .expect("Could not create pso");

        let (_, srv) = {
            let mut img_path = get_assets_folder().unwrap().to_path_buf();
            img_path.push(texture_name);
            let img = image::open(img_path)
                .expect("Could not open image")
                .to_rgba();
            let (iw, ih) = img.dimensions();
            let kind = Kind::D2(iw as u16, ih as u16, AaMode::Single);
            factory
                .create_texture_immutable_u8::<ColorFormat>(kind, &[&img])
                .expect("Could not create texture")
        };

        let sampler = factory.create_sampler_linear();

        let (verts, inds) = load_obj(model_name).expect("Could not load obj file");
        let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&verts[..], &inds[..]);
        let data = pipe::Data {
            vbuf: vertex_buffer,
            vert_locals: factory.create_constant_buffer(1),
            shared_locals: factory.create_constant_buffer(1),
            lights: factory.create_constant_buffer(MAX_LIGHTS),
            main_texture: (srv, sampler),
            out: rtv,
            main_depth: dsv,
        };
        
        let bundle = Bundle::new(slice, pso, data);
        let similarity = Similarity3::from_scaling(1.0);
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
    ) {
        let num_lights = lights.len() as u32;
        assert!(num_lights < MAX_LIGHTS as u32);
        encoder.update_constant_buffer(&self.bundle.data.shared_locals, &SharedLocals { num_lights });
        encoder
            .update_buffer(&self.bundle.data.lights, &lights, 0)
            .expect("Could not update buffer");
    }

    #[inline]
    pub fn update_views<W: WindowExt<R>>(&mut self, window: &W) {
        window.update_views(&mut self.bundle.data.out, &mut self.bundle.data.main_depth);
    }
}
