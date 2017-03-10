use gfx::{Device, Factory, Resources};
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx_device_gl::{Device as GlDevice, Factory as GlFactory, Resources as GlResources};
use winit;

mod gl;

pub use self::gl::launch_gl;

pub trait Window<R: Resources> {
    type SwapBuffersError;
    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError>;

    fn update_views<C, D>(&self,
                          rtv: &mut RenderTargetView<R, C>,
                          dsv: &mut DepthStencilView<R, D>)
        where C: RenderFormat,
              D: DepthFormat;

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

/*
#[cfg(target_os = "macos")]
mod metal;
*/

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LaunchError;

pub fn launch_native<C, D>(wb: winit::WindowBuilder)
                           -> Result<(impl Window<impl Resources>,
                                      impl Device,
                                      impl Factory<impl Resources>,
                                      RenderTargetView<impl Resources, C>,
                                      DepthStencilView<impl Resources, D>),
                                     LaunchError>
    where C: RenderFormat,
          D: DepthFormat {
    launch_gl(wb)
}

pub struct LaunchNativeError;

pub enum NativeOrGlFallback<R, W, D, F, GlW, Cf, Df>
    where R: Resources,
          W: Window<R>,
          D: Device,
          F: Factory<R>,
          GlW: Window<GlResources>,
          Cf: RenderFormat,
          Df: DepthFormat
{
    Native(W, D, F, RenderTargetView<R, Cf>, DepthStencilView<R, Df>),
    GlFallback {
        native_launch_error: LaunchError,
        fallback: (GlW, GlDevice, GlFactory,
                   RenderTargetView<GlResources, Cf>,
                   DepthStencilView<GlResources, Df>),
    }
}

pub enum Backend {
    Gl,
    Metal,
}
