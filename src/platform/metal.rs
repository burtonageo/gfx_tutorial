use super::{Backend, Window};
use winit::WindowBuilder;
use gfx::format::RenderFormat;
use gfx::handle::RenderTargetView;
use gfx_device_metal::{Device, Factory, Resources};
use gfx_window_metal::{init, MetalWindow, InitError};

impl Window for MetalWindow {
    type SwapBuffersError = ();

    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError> {
        MetalWindow::swap_buffers(self)
    }
}

pub fn init_metal<C: RenderFormat>(builder: WindowBuilder)
                                   -> Result<(MetalWindow,
                                              Device,
                                              Factory,
                                              RenderTargetView<Resources, C>),
                                             InitError> {
    unimplemented!()
}
