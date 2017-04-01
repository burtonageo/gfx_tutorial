#[macro_use]
extern crate gfx;
extern crate rusttype;

use gfx::{CombinedError, CommandBuffer, Encoder, PipelineStateError, Resources};
use gfx::handle::{Texture, RenderTargetView, ShaderResourceView};
use gfx::pso::bundle::Bundle;
use gfx::traits::FactoryExt;
use rusttype::gpu_cache::{Cache as GpuCache, CacheReadErr, CacheWriteErr};
use std::error::Error as StdError;
use std::fmt;

pub struct TextRenderer<R: Resources> {
    font_cache: GpuCache,
    // texture: Texture<R, T::Surface>,
    // srv: ShaderResourceView<R, T::View>,
    bundle: Bundle<R, pipe::Data<R>>,
    current_color: [f32; 4],
}

impl<R: Resources> TextRenderer<R> {
    pub fn new<F: FactoryExt<R>>(factory: &mut F,
                                 render_target: RenderTargetView<R, gfx::format::Rgba8>,
                                 width: u16,
                                 height: u16,
                                 scale_tolerance: f32,
                                 position_tolerance: f32)
                                 -> TextResult<Self> {
    	const PLANE: &[Vertex] = &[
            Vertex { pos: [ -1.0, -1.0, 0.0 ], tex: [0.0, 0.0] },
            Vertex { pos: [  1.0, -1.0, 0.0 ], tex: [0.0, 0.0] },
            Vertex { pos: [  1.0,  1.0, 0.0 ], tex: [0.0, 0.0] },
            Vertex { pos: [ -1.0,  1.0, 0.0 ], tex: [0.0, 0.0] },
    	];

        const INDICES: &[u16] = &[0u16, 1, 2, 2, 3, 0];

        const VERT_SRC: &[u8] = include_bytes!("../data/glsl/text.vs");
        const FRAG_SRC: &[u8] = include_bytes!("../data/glsl/text.fs");

        // let (t, srv, rtv) = factory.create_render_target(width, height)?;
        let pso = factory.create_pipeline_simple(VERT_SRC, FRAG_SRC, pipe::new())?;
        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(PLANE, INDICES);

        let data = pipe::Data {
            vbuf: vbuf,
            locals: factory.create_constant_buffer(1),
            out: render_target,
        };

        Ok(TextRenderer {
            font_cache: GpuCache::new(width as u32, height as u32, scale_tolerance, position_tolerance),
            // texture: t,
            // srv: srv,
            bundle: Bundle::new(slice, pso, data),
            current_color: Default::default(),
        })
    }

    #[inline]
    pub fn add_text(&mut self, text: &StyledText) {
        self.current_color = text.color.to_slice_rgba();
    }

    #[inline]
    pub fn font_cache(&self) -> &GpuCache {
        &self.font_cache
    }

    #[inline]
    pub fn encode<C: CommandBuffer<R>>(&self, encoder: &mut Encoder<R, C>) {
        encoder.update_constant_buffer(&self.bundle.data.locals, &Locals { text_color: self.current_color });
        self.bundle.encode(encoder);
    }
}

impl<R: Resources> fmt::Debug for TextRenderer<R> {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.debug_struct("TextRenderer")
            .field("font_cache", &"rusttype::gpu_cache::Cache { .. }")
            //.field("texture", &self.texture)
            //.field("srv", &self.srv)
            //.field("bundle", &self.bundle)
            .field("current_color", &self.current_color)
            .finish()
    }
}

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_Pos",
        tex: [f32; 2] = "v_Tex",
    }

    constant Locals {
        text_color: [f32; 4] = "f_TextColor",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        locals: gfx::ConstantBuffer<Locals> = "f_TextLocals",
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StyledText<'a> {
    pub string: &'a str,
    pub color: Color,
    pub font_size: u32,
    pub position: [f32; 3],
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn to_slice_rgba(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

pub type TextResult<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Gfx(CombinedError),
    GfxPso(PipelineStateError<String>),
    CacheRead(CacheReadErr),
    CacheWrite(CacheWriteErr),
}

impl fmt::Display for Error {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.pad(self.description())
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Gfx(_) => "an error occurred during a gfx operation",
            Error::GfxPso(_) => "could not create pso",
            Error::CacheRead(_) => "an error occurred when reading from the cache",
            Error::CacheWrite(_) => "an error occurred when writing to the cache",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Gfx(ref e) => Some(e),
            Error::GfxPso(ref e) => Some(e),
            _ => None,
        }
    }
}

impl From<CombinedError> for Error {
	#[inline]
    fn from(e: CombinedError) -> Self {
        Error::Gfx(e)
    }
}

impl From<PipelineStateError<String>> for Error {
	#[inline]
    fn from(e: PipelineStateError<String>) -> Self {
        Error::GfxPso(e)
    }
}

impl<'a> From<PipelineStateError<&'a str>> for Error {
	#[inline]
    fn from(e: PipelineStateError<&'a str>) -> Self {
        Error::GfxPso(e.into())
    }
}

impl From<CacheReadErr> for Error {
	#[inline]
    fn from(e: CacheReadErr) -> Self {
        Error::CacheRead(e)
    }
}

impl From<CacheWriteErr> for Error {
	#[inline]
    fn from(e: CacheWriteErr) -> Self {
        Error::CacheWrite(e)
    }
}
