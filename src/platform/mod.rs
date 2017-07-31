#![allow(dead_code)]

use gfx::{CommandBuffer, Encoder, Factory, Resources};
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use std::error::Error;

#[cfg(feature = "gl")]
mod gl;
#[cfg(feature = "gl")]
pub use self::gl::launch_gl;

#[cfg(all(target_os = "macos", feature = "metal"))]
mod metal;
#[cfg(all(target_os = "macos", feature = "metal"))]
pub use self::metal::launch_metal as launch_native;

#[cfg(all(target_os = "windows", feature = "dx11"))]
mod dx11;
#[cfg(all(target_os = "windows", feature = "dx11"))]
pub use self::dx11::launch_dx11 as launch_native;

#[cfg(all(feature = "gl", not(any(feature = "metal", feature = "dx11"))))]
pub use self::gl::launch_gl as launch_native;

#[derive(Debug, Default)]
pub struct ContextBuilder {
    pub is_vsync_enabled: bool,
}

impl ContextBuilder {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn with_vsync_enabled(self, vsync_enabled: bool) -> Self {
        ContextBuilder { is_vsync_enabled: vsync_enabled, ..self }
    }
}

#[cfg(not(any(feature = "gl", feature = "metal", feature = "dx11")))]
pub fn launch_native(wb: ::winit::WindowBuilder, we: &::winit::EventLoop) -> ! {
    panic!("No api selected")
}

pub trait WindowExt<R: Resources> {
    type SwapBuffersError: Error;
    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError>;

    fn update_views<C: RenderFormat, D: DepthFormat>(&self,
                                                     _rtv: &mut RenderTargetView<R, C>,
                                                     _dsv: &mut DepthStencilView<R, D>) { }
}

pub trait FactoryExt<R: Resources>: Factory<R> {
    type CommandBuffer: CommandBuffer<R>;
    fn create_encoder(&mut self) -> Encoder<R, Self::CommandBuffer>;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Backend {
    Gl,
    Metal,
    D3d11,
    Vulkan,
    #[doc(hidden)]
    __NonexhaustiveCheck,
}

impl Backend {
    pub fn is_gl(&self) -> bool {
        if let Backend::Gl = *self {
            true
        } else {
            false
        }
    }

    pub fn is_metal(&self) -> bool {
        if let Backend::Metal = *self {
            true
        } else {
            false
        }
    }

    pub fn is_d3d11(&self) -> bool {
        if let Backend::D3d11 = *self {
            true
        } else {
            false
        }
    }

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

