use super::{LaunchError, Window};
use gfx_device_gl::{Device, Factory, Resources};
use gfx_window_glutin;
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use glutin::{ContextError, Window as GlutinWindow, WindowBuilder as GlutinWindowBuilder};
use winit::WindowBuilder;

impl Window<Resources> for GlutinWindow {
    type SwapBuffersError = ContextError;

    #[inline]
    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError> {
        self.swap_buffers()
    }

    #[inline]
    fn update_views<C, D>(&self,
                          rtv: &mut RenderTargetView<Resources, C>,
                          dsv: &mut DepthStencilView<Resources, D>)
        where C: RenderFormat,
              D: DepthFormat {
        gfx_window_glutin::update_views(&self, rtv, dsv)
    }

    #[inline]
    fn set_title(&self, title: &str) {
        GlutinWindow::set_title(self, title)
    }

    #[inline]
    fn show(&self) {
        GlutinWindow::show(self)
    }

    #[inline]
    fn hide(&self) {
        GlutinWindow::hide(self)
    }

    #[inline]
    fn get_position(&self) -> Option<(i32, i32)> {
        GlutinWindow::get_position(self)
    }

    #[inline]
    fn set_position(&self, x: i32, y: i32) {
        GlutinWindow::set_position(self, x, y)
    }

    #[inline]
    fn get_inner_size(&self) -> Option<(u32, u32)> {
        GlutinWindow::get_inner_size(self)
    }

    #[inline]
    fn get_inner_size_points(&self) -> Option<(u32, u32)> {
        GlutinWindow::get_inner_size_points(self)
    }

    #[inline]
    fn get_inner_size_pixels(&self) -> Option<(u32, u32)> {
        GlutinWindow::get_inner_size_pixels(self)
    }

    #[inline]
    fn set_inner_size(&self, x: u32, y: u32) {
        GlutinWindow::set_inner_size(self, x, y)
    }

    #[inline]
    fn hidpi_factor(&self) -> f32 {
        GlutinWindow::hidpi_factor(self)
    }

    #[inline]
    fn set_cursor_position(&self, x: i32, y: i32)  -> Result<(), ()> {
        GlutinWindow::set_cursor_position(self, x, y)
    }
}

pub fn launch_gl<C, D>(wb: WindowBuilder)
                       -> Result<(GlutinWindow,
                                  Device,
                                  Factory,
                                  RenderTargetView<Resources, C>,
                                  DepthStencilView<Resources, D>),
                                 LaunchError>
    where C: RenderFormat,
          D: DepthFormat {
    let builder = GlutinWindowBuilder::from_winit_builder(wb);
    let (window, device, factory, main_color, main_depth) = gfx_window_glutin::init(builder);
    Ok((window, device, factory, main_color, main_depth))
}
