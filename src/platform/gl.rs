use super::{Backend, FactoryExt, Window, WinitWindowExt};
use gfx_device_gl::{CommandBuffer, Device, Factory, Resources};
use gfx_window_glutin;
use gfx::Encoder;
use gfx::format::{DepthFormat, RenderFormat};
use gfx::handle::{DepthStencilView, RenderTargetView};
use glutin::{ContextError, Window as GlutinWindow, WindowBuilder as GlutinWindowBuilder};
use void::Void;
use winit;

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
}

impl WinitWindowExt<Resources> for GlutinWindow {
    fn as_winit_window(&self) -> &winit::Window {
        self.as_winit_window()
    }
}

impl FactoryExt<Resources> for Factory {
    type CommandBuffer = CommandBuffer;

    fn create_encoder(&mut self) -> Encoder<Resources, Self::CommandBuffer> {
        self.create_command_buffer().into()
    }
}

pub fn launch_gl<C, D>(wb: winit::WindowBuilder)
                       -> Result<(Backend,
                                  GlutinWindow,
                                  Device,
                                  Factory,
                                  RenderTargetView<Resources, C>,
                                  DepthStencilView<Resources, D>),
                                 Void>
    where C: RenderFormat,
          D: DepthFormat {
    let (w, d, f, rtv, dst) = gfx_window_glutin::init(GlutinWindowBuilder::from_winit_builder(wb));
    Ok((Backend::Gl, w, d, f, rtv, dst))
}
