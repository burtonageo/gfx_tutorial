use gfx::{Device, Factory, Resources};
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use std::error::Error;
use std::fmt;
use void::Void;
use winit::{self, WindowBuilder};

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
                                                     rtv: &mut RenderTargetView<R, C>,
                                                     dsv: &mut DepthStencilView<R, D>);

    fn set_title(&self, title: &str);
    fn show(&self);
    fn hide(&self);
    fn get_position(&self) -> Option<(i32, i32)>;
    fn set_position(&self, x: i32, y: i32);
    fn get_inner_size(&self) -> Option<(u32, u32)>;
    fn get_inner_size_points(&self) -> Option<(u32, u32)>;
    fn get_inner_size_pixels(&self) -> Option<(u32, u32)>;
    fn set_inner_size(&self, x: u32, y: u32);
    fn hidpi_factor(&self) -> f32;
    fn set_cursor_position(&self, x: i32, y: i32) -> Result<(), ()>;
}

pub fn launch_native<C, D>(wb: winit::WindowBuilder)
                           -> Result<(impl Window<impl Resources>,
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

pub enum Backend {
    Gl,
    Metal,
    D3d11,
}
