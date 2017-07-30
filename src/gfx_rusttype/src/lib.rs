#[macro_use]
extern crate gfx;
extern crate rusttype;

use gfx::{CombinedError, CommandBuffer, Encoder, PipelineStateError, Resources, ResourceViewError,
          SHADER_RESOURCE, TRANSFER_DST, UpdateError};
use gfx::format::{ChannelType, Formatted, Rgba8, Swizzle};
use gfx::handle::{Texture, RenderTargetView};
use gfx::memory::Usage;
use gfx::pso::bundle::Bundle;
use gfx::texture::{AaMode, CreationError, NewImageInfo, Kind};
use gfx::traits::FactoryExt;
use rusttype::{FontCollection, Point, Scale};
use rusttype::gpu_cache::{Cache as GpuCache, CacheReadErr, CacheWriteErr};
use std::error::Error as StdError;
use std::fmt;

pub struct TextRenderer<R: Resources> {
    font_cache: GpuCache,
    texture: Texture<R, <Rgba8 as Formatted>::Surface>,
    bundle: Bundle<R, pipe::Data<R>>,
    current_color: [f32; 4],
    font_collection: FontCollection<'static>,
}

impl<R: Resources> TextRenderer<R> {
    pub fn new<F: FactoryExt<R>>(factory: &mut F,
                                 render_target: RenderTargetView<R, gfx::format::Rgba8>,
                                 width: u16,
                                 height: u16,
                                 scale_tolerance: f32,
                                 position_tolerance: f32,
                                 font_collection: FontCollection<'static>)
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

        let kind = Kind::D2(width, height, AaMode::Single);
        let t = factory.create_texture(kind,
                                       1,
                                       TRANSFER_DST | SHADER_RESOURCE,
                                       Usage::Dynamic,
                                       Some(ChannelType::Unorm))?;
        let srv = factory.view_texture_as_shader_resource::<Rgba8>(&t, (1, 1), Swizzle::new())?;
        let sampler = factory.create_sampler_linear();
        let pso = factory.create_pipeline_simple(VERT_SRC, FRAG_SRC, pipe::new())?;
        let (vbuf, slice) = factory.create_vertex_buffer_with_slice(PLANE, INDICES);

        let data = pipe::Data {
            vbuf: vbuf,
            locals: factory.create_constant_buffer(1),
            text_sampler: (srv, sampler),
            out: render_target,
        };

        Ok(TextRenderer {
            font_cache: GpuCache::new(width as u32, height as u32, scale_tolerance, position_tolerance),
            texture: t,
            bundle: Bundle::new(slice, pso, data),
            current_color: Default::default(),
            font_collection: font_collection,
        })
    }

    pub fn add_text<C: CommandBuffer<R>>(&mut self,
                                         text: &StyledText,
                                         texture_update_encoder: &mut Encoder<R, C>)
                                         -> TextResult<()> {
        self.current_color = text.color.to_slice_rgba();
        let fid = text.font_index;
        let font = self.font_collection.font_at(fid).ok_or(Error::FontNotFound(fid))?;

        for glyph in font.layout(text.string, text.scale, text.position) {
            self.font_cache.queue_glyph(fid, glyph);
        }

        let mut texture_update_error = None;
        let texture = &self.texture;

        self.font_cache.cache_queued(|rect, pix_data| {
            let data = pix_data.iter().map(|&byte| [255, 0, 0, byte]).collect::<Vec<_>>();
            let Point {x, y} = rect.min;
            let (w, h) = (rect.width() as u16, rect.height() as u16);
            let img_info = NewImageInfo {
                xoffset: x as u16,
                yoffset: y as u16,
                zoffset: 0,
                width: w,
                height: h,
                depth: 0,
                format: (),
                mipmap: 1,
            };
            texture_update_error = texture_update_encoder.update_texture::<_, Rgba8>(texture,
                                                                                     None,
                                                                                     img_info,
                                                                                     &data[..]).err();
        })?;

        if let Some(res) = texture_update_error { Err(From::from(res)) } else { Ok(()) }
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
            .field("texture", &self.texture)
            .field("bundle", &"gfx::pso::bundle::Bundle { .. }")
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
        text_sampler: gfx::TextureSampler<[f32; 4]> = "f_TextSampler",
        locals: gfx::ConstantBuffer<Locals> = "f_TextLocals",
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StyledText<'a> {
    pub string: &'a str,
    pub color: Color,
    pub font_index: usize,
    pub scale: Scale,
    pub position: Point<f32>,
}

impl<'a> StyledText<'a> {
    #[inline]
    pub fn new(s: &'a str, color: Color, scale: Scale, position: Point<f32>) -> Self {
        StyledText {
            string: s,
            color: color,
            font_index: Default::default(),
            scale: scale,
            position: position,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    #[inline]
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color {
            r: r,
            g: g,
            b: b,
            a: a,
        }
    }

    #[inline]
    pub fn to_slice_rgba(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

pub type TextResult<T> = ::std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Gfx(CombinedError),
    TextureUpdate(UpdateError<[u16; 3]>),
    TextureCreate(CreationError),
    Pso(PipelineStateError<String>),
    ResourceView(ResourceViewError),
    CacheRead(CacheReadErr),
    CacheWrite(CacheWriteErr),
    FontNotFound(usize),
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        let desc = self.description();
        match *self {
            Error::Gfx(ref e) => writeln!(fmtr, "{}: {}", e, desc),
            Error::Pso(ref e) => writeln!(fmtr, "{}: {}", e, desc),
            Error::TextureUpdate(ref e) => writeln!(fmtr, "{:?}: {}", e, desc),
            Error::TextureCreate(ref e) => writeln!(fmtr, "{}: {}", e, desc),
            Error::ResourceView(ref e) => writeln!(fmtr, "{}: {}", e, desc),
            Error::CacheRead(ref e) => writeln!(fmtr, "{:?}: {}", e, desc),
            Error::CacheWrite(ref e) => writeln!(fmtr, "{:?}: {}", e, desc),
            Error::FontNotFound(ref e) => writeln!(fmtr, "{}: {}", e, desc),
        }
    }
}

impl StdError for Error {
    #[inline]
    fn description(&self) -> &str {
        match *self {
            Error::Gfx(_) => "an error occurred during a gfx operation",
            Error::Pso(_) => "could not create pso",
            Error::TextureUpdate(_) => "an error occurred while updating the texture",
            Error::TextureCreate(_) => "could not create glyph texture",
            Error::ResourceView(_) => "an error occurred while creating an srv from the texture",
            Error::CacheRead(_) => "an error occurred when reading from the cache",
            Error::CacheWrite(_) => "an error occurred when writing to the cache",
            Error::FontNotFound(_) => "the font could not be found at the index"
        }
    }

    #[inline]
    fn cause(&self) -> Option<&StdError> {
        match *self {
            Error::Gfx(ref e) => Some(e),
            Error::Pso(ref e) => Some(e),
            Error::TextureCreate(ref e) => Some(e),
            Error::ResourceView(ref e) => Some(e),
            // Error::TextureUpdate(ref e) => Some(e),
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
        Error::Pso(e)
    }
}

impl<'a> From<PipelineStateError<&'a str>> for Error {
	#[inline]
    fn from(e: PipelineStateError<&'a str>) -> Self {
        Error::Pso(e.into())
    }
}

impl From<UpdateError<[u16; 3]>> for Error {
    #[inline]
    fn from(e: UpdateError<[u16; 3]>) -> Self {
        Error::TextureUpdate(e)
    }
}

impl From<CreationError> for Error {
    #[inline]
    fn from(e: CreationError) -> Self {
        Error::TextureCreate(e)
    }
}

impl From<ResourceViewError> for Error {
    #[inline]
    fn from(e: ResourceViewError) -> Self {
        Error::ResourceView(e)
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
