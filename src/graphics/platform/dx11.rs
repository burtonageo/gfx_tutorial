use super::{Backend, ContextBuilder, FactoryExt, WindowExt};
use gfx::{CombinedError, Encoder};
use gfx::format::{DepthFormat, Formatted, RenderFormat, TextureChannel, TextureSurface};
use gfx::handle::{DepthStencilView, RenderTargetView};
use gfx::texture::Size;
use gfx_device_dx11::{CommandBuffer, CommandList, Device, Factory, Resources};
use gfx_window_dxgi::{init, InitError, Window as DxgiWindow};
use std::error::Error;
use std::fmt;
use winit;

impl WindowExt<Resources> for DxgiWindow {
    type SwapBuffersError = !;
    fn swap_buffers(&self) -> Result<(), Self::SwapBuffersError> {
        DxgiWindow::swap_buffers(self, 0u8);
        Ok(())
    }
}

impl FactoryExt<Resources> for Factory {
    type CommandBuffer = CommandBuffer<CommandList>;
    #[inline]
    fn create_encoder(&mut self) -> Encoder<Resources, Self::CommandBuffer> {
        self.create_command_buffer().into()
    }
}

pub fn launch_dx11<C, D>(
    wb: winit::WindowBuilder,
    el: &winit::EventsLoop,
    _: ContextBuilder,
) -> Result<
    (Backend,
     DxgiWindow,
     Device,
     Factory,
     RenderTargetView<Resources, C>,
     DepthStencilView<Resources, D>),
    InitError,
>
where
    C: RenderFormat,
    D: DepthFormat,
    <D as Formatted>::Channel: TextureChannel,
    <D as Formatted>::Surface: TextureSurface,
{
    /*
    let (window, device, mut factory, rtv) = init(wb, el)?;
    let (w, h) = window.get_inner_size_points().unwrap_or((0, 0));
    let dsv = factory.create_depth_stencil_view_only(w as Size, h as Size)?;
    Ok((Backend::D3d11, window, device, factory, rtv, dsv))
    */
    unimplemented!();
}
