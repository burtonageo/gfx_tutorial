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
pub use self::metal::launch_metal as launch_native;

#[cfg(target_os = "windows")]
mod dx11;
#[cfg(target_os = "windows")]
pub use self::metal::launch_dx11 as launch_native;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
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

