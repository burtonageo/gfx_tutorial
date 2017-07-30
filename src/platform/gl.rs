use super::{Backend, FactoryExt, WindowExt};
use gfx_device_gl::{CommandBuffer, Device, Factory, Resources};
use gfx_window_glutin;
use gfx::Encoder;
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use glutin::{ContextBuilder, ContextError, GlWindow, WindowBuilder as GlutinWindowBuilder};
use void::Void;
use winit;

impl WindowExt<Resources> for GlWindow {
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
}

impl FactoryExt<Resources> for Factory {
    type CommandBuffer = CommandBuffer;

    fn create_encoder(&mut self) -> Encoder<Resources, Self::CommandBuffer> {
        self.create_command_buffer().into()
    }
}

pub fn launch_gl<C, D>(wb: winit::WindowBuilder, el: &winit::EventsLoop)
                       -> Result<(Backend,
                                  GlWindow,
                                  Device,
                                  Factory,
                                  RenderTargetView<Resources, C>,
                                  DepthStencilView<Resources, D>),
                                 Void>
    where C: RenderFormat,
          D: DepthFormat {

    let (w, d, f, rtv, dst) = gfx_window_glutin::init(wb, ContextBuilder::new(), el);
    Ok((Backend::Gl, w, d, f, rtv, dst))
}
