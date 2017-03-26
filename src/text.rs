use gfx::{CombinedError, CommandBuffer, Encoder, Factory, PipelineState, PipelineStateError, Resources, self};
use gfx::format::TextureFormat;
use gfx::handle::{Texture, ShaderResourceView};
use gfx::texture::{AaMode, Kind};
use rusttype::gpu_cache::{Cache as GpuCache, CacheReadErr, CacheWriteErr};
use std::error::Error as StdError;
use std::fmt;

pub struct TextRenderer<R: Resources, T: TextureFormat, M> {
	font_cache: GpuCache,
	texture: Texture<R, T::Surface>,
	srv: ShaderResourceView<R, T::View>,
	pso: PipelineState<R, M>,
}

impl<R: Resources, T: TextureFormat, M> TextRenderer<R, T, M> {
	pub fn new<F>(factory: &mut F, aa: AaMode, width: u16, height: u16, scale_tolerance: f32, position_tolerance: f32) -> Result<Self>
		where
			F: Factory<R>
	{
		let raw_texture_data = vec![0u8; width as usize * height as usize];
		let (t, srv) = factory.create_texture_immutable_u8::<T>(Kind::D2(width, height, aa), &[&raw_texture_data])?;

		let renderer: Result<TextRenderer<R, T, M>> = Ok(TextRenderer {
			font_cache: GpuCache::new(width as u32, height as u32, scale_tolerance, position_tolerance),
			texture: t,
			srv: srv,
			pso: unimplemented!(),
		});

		renderer
	}

	#[inline]
	pub fn font_cache(&self) -> &GpuCache {
		&self.font_cache
	}

	pub fn encode<C: CommandBuffer<R>>(&self, encoder: &mut Encoder<R, C>) -> Result<()> {
		unimplemented!();
	}
}


impl<R, T, M> fmt::Debug for TextRenderer<R, T, M>
	where
		R: Resources,
		T: TextureFormat,
		T::View: fmt::Debug,
		T::Surface: fmt::Debug,
{
	fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
		fmtr.debug_struct("TextRenderer")
			.field("font_cache", &"rusttype::gpu_cache::Cache { .. }")
			.field("texture", &self.texture)
			.field("srv", &self.srv)
			.finish()
	}
}


gfx_defines! {
	vertex Vertex {
		pos: [f32; 3] = "v_pos",
		tex: [f32; 2] = "v_tex",
	}

	/*
	pipeline pipe {
		vbuf: gfx::VertexBuffer<Vertex> = (),
		out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
        main_depth: gfx::DepthTarget<gfx::format::Depth> = gfx::preset::depth::LESS_EQUAL_WRITE,
	}
	*/
}

pub type Result<T> = ::std::result::Result<T, Error>;

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
		"an error occurred"
	}
}

impl From<CombinedError> for Error {
	fn from(e: CombinedError) -> Self {
		Error::Gfx(e)
	}
}

impl From<PipelineStateError<String>> for Error {
	fn from(e: PipelineStateError<String>) -> Self {
		Error::GfxPso(e)
	}
}

impl<'a> From<PipelineStateError<&'a str>> for Error {
	fn from(e: PipelineStateError<&'a str>) -> Self {
		Error::GfxPso(e.into())
	}
}

impl From<CacheReadErr> for Error {
	fn from(e: CacheReadErr) -> Self {
		Error::CacheRead(e)
	}
}

impl From<CacheWriteErr> for Error {
	fn from(e: CacheWriteErr) -> Self {
		Error::CacheWrite(e)
	}
}
