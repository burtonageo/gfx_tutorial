#![allow(dead_code)]

use gfx::{Device, Factory, Resources};
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use std::error::Error;
use std::fmt;
use void::Void;
use winit;

mod gl;
pub use self::gl::launch_gl;

/*
#[cfg(target_os = "macos")]
mod metal;
#[cfg(target_os = "macos")]
use self::metal::launch_metal as launch_native;
*/

pub trait Window<R: Resources> {
    type SwapBuffersError: Error;
    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError>;

    fn update_views<C: RenderFormat, D: DepthFormat>(&self,
                                                     _rtv: &mut RenderTargetView<R, C>,
                                                     _dsv: &mut DepthStencilView<R, D>) { }
}

pub trait WinitWindowExt<R: Resources>: Window<R> {
    fn as_winit_window(&self) -> &winit::Window;
    fn as_winit_window_mut(&mut self) -> &mut winit::Window;
}

pub fn launch_native<C, D>(wb: winit::WindowBuilder)
                           -> Result<(Backend,
                                      impl WinitWindowExt<impl Resources>,
                                      impl Device,
                                      impl Factory<impl Resources>,
                                      RenderTargetView<impl Resources, C>,
                                      DepthStencilView<impl Resources, D>),
                                     LaunchError>
    where C: RenderFormat,
          D: DepthFormat {
    Ok(launch_gl(wb)?)
}

#[derive(Debug)]
pub struct LaunchError(Box<Error>);

impl fmt::Display for LaunchError {
    #[inline]
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.0.as_ref(), fmtr)
    }
}

impl Error for LaunchError {
    #[inline]
    fn description(&self) -> &str {
        "could not create gfx window or GPU connection"
    }

    #[inline]
    fn cause(&self) -> Option<&Error> {
        Some(self.0.as_ref())
    }
}

impl From<Box<Error>> for LaunchError {
    #[inline]
    fn from(e: Box<Error>) -> LaunchError {
        LaunchError(e)
    }
}

impl From<Void> for LaunchError {
    #[cold]
    fn from(_: Void) -> LaunchError {
        unreachable!()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Backend(BackendInner);

impl Backend {
    #[inline]
    pub fn gl() -> Self {
        Backend(BackendInner::Gl)
    }

    #[inline]
    pub fn metal() -> Self {
        Backend(BackendInner::Metal)
    }

    #[inline]
    pub fn d3d11() -> Self {
        Backend(BackendInner::D3d11)
    }

    pub fn select<'a>(&self, _shaders: Shaders<'a>) -> ShaderPipeline<'a> {
        use self::BackendInner::*;
        match self.0 {
            Gl => unimplemented!(),
            Metal => unimplemented!(),
            D3d11 => unimplemented!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum BackendInner {
    Gl,
    Metal,
    D3d11,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Shaders<'a> {
    D3d11(ShaderPipeline<'a>),
    Glsl150(ShaderPipeline<'a>),
    Glsl120(ShaderPipeline<'a>),
    Metal(ShaderPipeline<'a>),
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

