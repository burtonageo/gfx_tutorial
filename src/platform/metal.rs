use super::{Backend, FactoryExt, Window, WinitWindowExt};
use winit;
use gfx::{CombinedError, Encoder, Factory};
use gfx::format::{DepthFormat, Formatted, RenderFormat, TextureChannel, TextureSurface};
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx::texture::Size;
use gfx_device_metal::{CommandBuffer, Device, Factory as MetalFactory, Resources};
use gfx_window_metal::{init, MetalWindow, InitError};
use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct MetalSwapBuffersError(());

impl fmt::Display for MetalSwapBuffersError {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.pad(self.description())
    }
}

impl Error for MetalSwapBuffersError {
    fn description(&self) -> &str {
        "an error occurred while swapping the window buffers"
    }
}

impl Window<Resources> for MetalWindow {
    type SwapBuffersError = MetalSwapBuffersError;

    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError> {
        MetalWindow::swap_buffers(self).map_err(|()| MetalSwapBuffersError(()))
    }
}

impl WinitWindowExt<Resources> for MetalWindow {
    fn as_winit_window(&self) -> &winit::Window {
        &*self
    }
}

impl FactoryExt<Resources> for MetalFactory {
    type CommandBuffer = CommandBuffer;
    fn create_encoder(&mut self) -> Encoder<Resources, Self::CommandBuffer> {
        self.create_command_buffer().into()
    }
}


#[derive(Debug)]
pub enum MetalInitError {
    Init(InitError),
    Dsv(CombinedError)
}

impl fmt::Display for MetalInitError {
    fn fmt(&self, fmtr: &mut fmt::Formatter) -> fmt::Result {
        fmtr.pad(self.description())
    }
}

impl Error for MetalInitError {
    fn description(&self) -> &str {
        "an error occurred while creating the metal window"
    }

    fn cause(&self) -> Option<&Error> {
        if let MetalInitError::Dsv(ref e) = *self {
            Some(e)
        } else {
            None
        }
    }
}

impl From<InitError> for MetalInitError {
    fn from(e: InitError) -> Self {
        MetalInitError::Init(e)
    }
}

impl From<CombinedError> for MetalInitError {
    fn from(e: CombinedError) -> Self {
        MetalInitError::Dsv(e)
    }
}

pub fn launch_metal<C, D>(wb: winit::WindowBuilder)
                          -> Result<(Backend,
                                     MetalWindow,
                                     Device,
                                     MetalFactory,
                                     RenderTargetView<Resources, C>,
                                     DepthStencilView<Resources, D>),
                                    MetalInitError>
    where C: RenderFormat,
          D: DepthFormat,
           <D as Formatted>::Channel: TextureChannel,
           <D as Formatted>::Surface: TextureSurface, {
    let (window, device, mut factory, rtv) = init(wb)?;
    let (w, h) = window.get_inner_size_points().unwrap_or((0, 0));
    let dsv = factory.create_depth_stencil_view_only(w as Size, h as Size)?;
    Ok((Backend::Metal, window, device, factory, rtv, dsv))
}