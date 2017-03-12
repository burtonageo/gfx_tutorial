#![allow(dead_code)]

use gfx::{CommandBuffer, Encoder, Factory, Resources};
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use std::error::Error;
use winit;

mod gl;
pub use self::gl::launch_gl;

#[cfg(target_os = "macos")]
mod metal;
#[cfg(target_os = "macos")]
pub use self::metal::launch_metal;
#[cfg(target_os = "macos")]
pub use self::metal::launch_metal as launch_native;

#[cfg(not(target_os = "macos"))]
pub use self::gl::launch_gl as launch_native;

pub trait Window<R: Resources> {
    type SwapBuffersError: Error;
    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError>;

    fn update_views<C: RenderFormat, D: DepthFormat>(&self,
                                                     _rtv: &mut RenderTargetView<R, C>,
                                                     _dsv: &mut DepthStencilView<R, D>) { }
}

pub trait WinitWindowExt<R: Resources>: Window<R> {
    fn as_winit_window(&self) -> &winit::Window;
}

pub trait FactoryExt<R: Resources>: Factory<R> {
    type CommandBuffer: CommandBuffer<R>;
    fn create_encoder(&mut self) -> Encoder<R, Self::CommandBuffer>;
}
/*
pub fn launch_native<C, D>(wb: winit::WindowBuilder)
                           -> Result<(Backend,
                                      impl WinitWindowExt<impl Resources>,
                                      impl Device,
                                      impl FactoryExt<impl Resources>,
                                      RenderTargetView<impl Resources, C>,
                                      DepthStencilView<impl Resources, D>),
                                     impl Error>
    where C: RenderFormat,
          D: DepthFormat,
          <D as Formatted>::Channel: TextureChannel,
          <D as Formatted>::Surface: TextureSurface {
    #[cfg(target_os = "macos")] { self::metal::launch_metal(wb) }

    #[cfg(not(target_os = "macos"))] { launch_gl(wb) }
}
*/

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Backend {
    Gl,
    Metal,
    D3d11,
    #[doc(hidden)]
    __NonexhaustiveCheck,
}

impl Backend {
    pub fn select<'a>(&self, shaders: Shaders<'a>) -> ShaderPipeline<'a> {
        use self::Backend::*;
        match *self {
            Gl => shaders.glsl150,
            Metal => shaders.metal,
            D3d11 => shaders.d3d11,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Shaders<'a> {
    pub d3d11: ShaderPipeline<'a>,
    pub glsl150: ShaderPipeline<'a>,
    pub glsl120: ShaderPipeline<'a>,
    pub metal: ShaderPipeline<'a>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShaderPipeline<'a> {
    Simple { vertex: &'a [u8], pixel: &'a [u8] },
    Geometry {
        vertex: &'a [u8],
        geometry: &'a [u8],
        pixel: &'a [u8]
    },
    Full {
        vertex: &'a [u8],
        tess_control: &'a [u8],
        tess_evaluation: &'a [u8],
        geometry: &'a [u8],
        pixel: &'a [u8],
    },
}

