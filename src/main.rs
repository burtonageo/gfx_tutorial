#![feature(conservative_impl_trait, never_type)]

extern crate alga;
extern crate ang;
extern crate find_folder;
#[macro_use]
extern crate gfx;
extern crate gfx_rusttype;
extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate num;
extern crate nalgebra as na;
extern crate rusttype;
#[macro_use]
extern crate scopeguard;
extern crate time;
extern crate void;
extern crate wavefront_obj;
extern crate winit;

#[cfg(feature = "gl")]
extern crate gfx_device_gl;
#[cfg(feature = "gl")]
extern crate gfx_window_glutin;
#[cfg(feature = "gl")]
extern crate glutin;

#[cfg(all(target_os = "macos", feature = "metal"))]
extern crate gfx_window_metal;
#[cfg(all(target_os = "macos", feature = "metal"))]
extern crate gfx_device_metal;

#[cfg(all(target_os = "windows", feature = "dx11"))]
extern crate gfx_window_dxgi;
#[cfg(all(target_os = "windows", feature = "dx11"))]
extern crate gfx_device_dx11;

mod load;
mod platform;
mod util;

use ang::{Angle, Degrees};
use gfx::{Bundle, CommandBuffer, Device, Encoder, Factory, Resources};
use gfx::format::{RenderFormat, Rgba8};
use gfx::handle::RenderTargetView;
use gfx::texture::{AaMode, Kind};
use gfx::traits::FactoryExt;
use gfx_rusttype::{Color, read_fonts, TextRenderer, StyledText};
use load::load_obj;
use na::{Isometry3, Perspective3, Point3, PointBase, Rotation3, Vector3};
use num::{cast, NumCast, One, Zero};
use platform::{ContextBuilder, FactoryExt as PlFactoryExt, WindowExt as PlatformWindow};
use std::env::args;
use std::ops::{Div, Neg};
use std::time::Duration as StdDuration;
use time::{Duration, PreciseTime};
use util::{get_assets_folder, open_file_relative_to_assets};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "position",
        uv: [f32; 2] = "tex_coord",
        normal: [f32; 3] = "normal",
    }

    constant ShaderLight {
        col: [f32; 4] = "color",
        pos: [f32; 3] = "position",
        power: f32 = "power",
    }

    constant VertLocals {
        transform: [[f32; 4]; 4] = "mvp_transform",
        model: [[f32; 4]; 4] = "model_transform",
        view: [[f32; 4]; 4] = "view_transform",
    }

    constant SharedLocals {
        num_lights: u32 = "num_lights",
    }

    constant Camera {
        position: [f32; 4] = "cam_position",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        vert_locals: gfx::ConstantBuffer<VertLocals> = "vert_locals",
        shared_locals: gfx::ConstantBuffer<SharedLocals> = "shared_locals",
        main_texture: gfx::TextureSampler<[f32; 4]> = "color_texture",
        lights: gfx::ConstantBuffer<ShaderLight> = "lights_array",
        camera: gfx::ConstantBuffer<Camera> = "main_camera",
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
        main_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

const GLSL_VERT_SRC: &'static [u8] = include_bytes!("../data/shader/glsl/standard.vs");
const GLSL_FRAG_SRC: &'static [u8] = include_bytes!("../data/shader/glsl/standard.fs");

const MSL_VERT_SRC: &'static [u8] = include_bytes!("../data/shader/msl/standard.vs");
const MSL_FRAG_SRC: &'static [u8] = include_bytes!("../data/shader/msl/standard.fs");

const CLEAR_COLOR: [f32; 4] = [0.005, 0.005, 0.1, 1.0];

#[derive(Debug)]
struct Input {
    position: Point3<f32>,
    horizontal_angle: Angle<f32>,
    vertical_angle: Angle<f32>,
    fov: Angle<f32>,
}

impl Input {
    #[inline]
    fn new() -> Self {
        Input {
            position: Point3::new(0.0, 0.0, 10.0),
            horizontal_angle: Angle::zero(),
            vertical_angle: Angle::zero(),
            fov: Angle::eighth(),
        }
    }
}

const SPEED: f32 = 4.0;
const MOUSE_SPEED: f32 = 7.0;

#[derive(Clone, Debug, PartialEq)]
struct Light {
    position: Point3<f32>,
    color: [f32; 4],
    power: f32,
}

impl Default for Light {
    #[inline]
    fn default() -> Self {
        Light {
            position: na::origin(),
            color: [na::zero(); 4],
            power: na::zero(),
        }
    }
}

impl From<Light> for ShaderLight {
    #[inline]
    fn from(l: Light) -> Self {
        let na::coordinates::XYZ { x, y, z } = *l.position;
        ShaderLight {
            pos: [x, y, z],
            col: l.color,
            power: l.power,
        }
    }
}

const MAX_LIGHTS: usize = 10;

fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let (win_w, win_h) = winit::get_primary_monitor().get_dimensions();
    let builder = winit::WindowBuilder::new()
        .with_dimensions(win_w, win_h)
        .with_title("Gfx Example");

    let (backend, window, mut device, mut factory, main_color, main_depth) =
        platform::launch_gl::<Rgba8, gfx::format::DepthStencil>(
                builder,
                &events_loop,
                ContextBuilder::new().with_vsync_enabled(true),
            )
            .expect("Could not create window or graphics device");

    window.hide_and_grab_cursor().expect("Could not set cursor state");
    window.center_cursor().expect("Could not set cursor position");

    let mut encoder = factory.create_encoder();
    let program = if backend.is_gl() {
        factory.link_program(GLSL_VERT_SRC, GLSL_FRAG_SRC).unwrap()
    } else {
        factory.link_program(MSL_VERT_SRC, MSL_FRAG_SRC).unwrap()
    };

    let pso = factory.create_pipeline_from_program(&program,
                                      gfx::Primitive::TriangleList,
                                      gfx::state::Rasterizer::new_fill().with_cull_back(),
                                      pipe::new())
        .expect("Could not create pso");

    let (_, srv) = {
        let mut img_path = get_assets_folder().unwrap().to_path_buf();
        img_path.push("img/checker.png");
        let img = image::open(img_path).expect("Could not open image").to_rgba();
        let (iw, ih) = img.dimensions();
        let kind = Kind::D2(iw as u16, ih as u16, AaMode::Single);
        factory.create_texture_immutable_u8::<Rgba8>(kind, &[&img])
            .expect("Could not create texture")
    };

    let sampler = factory.create_sampler_linear();

    let (verts, inds) = load_obj(&args().nth(1).unwrap_or("suzanne".into())).expect("Could not load obj file");
    let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&verts[..], &inds[..]);
    let data = pipe::Data {
        vbuf: vertex_buffer,
        vert_locals: factory.create_constant_buffer(1),
        shared_locals: factory.create_constant_buffer(1),
        lights: factory.create_constant_buffer(MAX_LIGHTS),
        camera: factory.create_constant_buffer(1),
        main_texture: (srv, sampler),
        out: main_color,
        main_depth: main_depth,
    };

    let mut fps = FpsRenderer::new(factory).expect("Could not create text renderer");
    let mut bundle = Bundle::new(slice, pso, data);

    let mut rot = Rotation3::identity();
    let mut iput = Input::new();
    let mut projection = Perspective3::new(window.aspect(), iput.fov.in_radians(), 0.1, 100.0);
    let mut last = PreciseTime::now();
    let mut is_paused = false;

    let mut show_fps = false;
    let mut is_running = true;

    while is_running {
        let current = PreciseTime::now();
        let dt = last.to(current);
        let dt_s = dt.num_nanoseconds().unwrap_or(0) as f32 / 1_000_000_000.0f32;
        last = current;

        defer!({
            let sleep_time = Duration::milliseconds(12)
                .checked_sub(&dt)
                .unwrap_or(Duration::zero())
                .to_std()
                .unwrap_or(StdDuration::from_millis(0));
            std::thread::sleep(sleep_time);
        });

        let direction = Vector3::new(iput.vertical_angle.cos() * iput.horizontal_angle.sin(),
                                     iput.vertical_angle.sin(),
                                     iput.vertical_angle.cos() * iput.horizontal_angle.cos());

        let right = Vector3::new((iput.horizontal_angle - Angle::quarter()).sin(),
                                 na::zero(),
                                 (iput.horizontal_angle - Angle::quarter()).cos());

        // Hack to get around lack of resize event on MacOS
        // https://github.com/tomaka/winit/issues/39
        if cfg!(target_os = "macos") {
            static mut WINDOW_LAST_W: i32 = 0;
            static mut WINDOW_LAST_H: i32 = 0;
            let (w, h) = window.windowext_get_inner_size();
            unsafe {
                if w != WINDOW_LAST_W || h != WINDOW_LAST_H {
                    window.update_views(&mut bundle.data.out, &mut bundle.data.main_depth);
                    projection.set_aspect(window.aspect());
                    WINDOW_LAST_W = w;
                    WINDOW_LAST_H = h;
                }
            }
        }

        events_loop.poll_events(|event| {
            use winit::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::Closed |
                        WindowEvent::KeyboardInput { input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. }, .. } => {
                            is_running = false;
                        }
                        #[cfg(not(target_os = "macos"))]
                        WindowEvent::Resized(..) => {
                            window.update_views(&mut bundle.data.out, &mut bundle.data.main_depth);
                            projection.set_aspect(window.aspect());
                        }
                        WindowEvent::MouseMoved { position: (x, y), .. } => {
                            let (ww, wh) = window.windowext_get_inner_size::<i32>();
                            let hidpi = window.hidpi_factor() as f64;

                            iput.horizontal_angle += Degrees(MOUSE_SPEED * dt_s * ((ww / 2) as f32 - (x / hidpi) as f32));
                            iput.vertical_angle -= Degrees(MOUSE_SPEED * dt_s * ((wh / 2) as f32 - (y / hidpi) as f32));

                            iput.horizontal_angle = iput.horizontal_angle.normalized();

                            let threshold = Angle::quarter() - Degrees(1.0f32);

                            if iput.vertical_angle > threshold {
                                iput.vertical_angle = threshold;
                            }

                            if iput.vertical_angle < threshold.neg() {
                                iput.vertical_angle = threshold.neg();
                            }

                            window.center_cursor().expect("Could not set cursor position");
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            match input {
                                KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Up), .. } => {
                                    iput.position -= direction * SPEED * dt_s;
                                }
                                KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Down), .. } => {
                                    iput.position += direction * SPEED * dt_s;
                                }
                                KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Left), .. } => {
                                    iput.position += right * SPEED * dt_s;
                                }
                                KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Right), .. } => {
                                    iput.position -= right * SPEED * dt_s;
                                }
                                KeyboardInput { state: ElementState::Released, virtual_keycode: Some(VirtualKeyCode::Space), .. } => {
                                    show_fps = !show_fps;
                                }
                                _ => {}
                            }
                        }
                        WindowEvent::Focused(gained) => {
                            is_paused = !gained;
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
        });

        if is_paused {
            continue;
        }

        rot *= Rotation3::new(Vector3::new(0.0, Degrees(25.0 * dt_s).in_radians(), 0.0));

        let view = {
            let up = right.cross(&direction);
            Isometry3::look_at_lh(&iput.position,
                                  &PointBase::from_coordinates(iput.position.coords + direction),
                                  &up)
        };

        encoder.clear(&bundle.data.out, CLEAR_COLOR);
        encoder.clear_depth(&bundle.data.main_depth, 1.0);

        let view_mat = view.to_homogeneous();
        let model_mat = rot.to_homogeneous();
        let mvp = projection.to_homogeneous() * view_mat * model_mat;
        let l0 = Light {
            position: Point3::new(0.0, 0.0, 3.0),
            color: [0.1, 0.1, 1.0, 1.0],
            power: 200.0,
        };
        let l1 = Light {
            position: Point3::new(0.0, 0.0, -2.0),
            color: [1.0, 0.0, 0.0, 1.0],
            power: 300.0,
        };
        let l2 = Light {
            position: Point3::new(-3.0, 1.0, 0.0),
            color: [0.0, 1.0, 0.0, 1.0],
            power: 80.0,
        };
        let lights: [ShaderLight; 3] = [l0.into(), l1.into(), l2.into()];
        encoder.update_constant_buffer(&bundle.data.vert_locals,
                                       &VertLocals {
                                           transform: *(mvp).as_ref(),
                                           model: *(model_mat).as_ref(),
                                           view: *(view_mat).as_ref(),
                                       });
        let cam_pos = [iput.position.x, iput.position.y, iput.position.z, 1.0];
        encoder.update_constant_buffer(&bundle.data.shared_locals,
                                       &SharedLocals { num_lights: lights.len() as u32 });
        encoder.update_constant_buffer(&bundle.data.camera, &Camera { position: cam_pos });
        encoder.update_buffer(&bundle.data.lights, &lights, 0).expect("Could not update buffer");

        bundle.encode(&mut encoder);
        fps.render(dt_s, &mut encoder, &bundle.data.out);
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

trait WindowExt {
   fn center_cursor(&self) -> Result<(), ()>;
    fn hide_and_grab_cursor(&self) -> Result<(), String>;
    fn windowext_get_inner_size<N: NumCast + Zero + Default>(&self) -> (N, N);
    fn aspect<N: Default + Div<Output=N> + NumCast + Zero + One>(&self) -> N {
        let (w, h) = self.windowext_get_inner_size::<N>();
        w / h
    }
}

impl WindowExt for winit::Window {
    fn center_cursor(&self) -> Result<(), ()> {
        let (ww, wh) = self.windowext_get_inner_size::<i32>();
        self.set_cursor_position(ww / 2, wh / 2)
    }

    fn hide_and_grab_cursor(&self) -> Result<(), String> {
        self.set_cursor_state(winit::CursorState::Hide)?;
        self.set_cursor_state(winit::CursorState::Grab)
    }

    fn windowext_get_inner_size<N: NumCast + Zero + Default>(&self) -> (N, N) {
        fn cast_pair<N: NumCast + Zero>((x, y): (u32, u32)) -> (N, N) {
            (cast(x).unwrap_or(Zero::zero()), cast(y).unwrap_or(Zero::zero()))
        }

        self.get_inner_size_pixels()
            .map(cast_pair)
            .unwrap_or(Default::default())
    }
}

#[cfg(not(windows))]
struct FpsRenderer<R: Resources, F: Factory<R>> {
    pub show_fps: bool,
    fps_string: String,
    text_renderer: gfx_text::Renderer<R, F>,
}

#[cfg(not(windows))]
impl<R: Resources, F: Factory<R>> FpsRenderer<R, F> {
    #[inline]
    fn new(factory: F) -> Result<Self, gfx_text::Error> {
        /*
        let mut text_renderer = {
            const POS_TOLERANCE: f32 = 0.1;
            const SCALE_TOLERANCE: f32 = 0.1;
            let (w, h) = window.windowext_get_inner_size::<u16>();
            TextRenderer::new(&mut factory, data.out.clone(), w, h, POS_TOLERANCE, SCALE_TOLERANCE,
                            read_fonts("fonts/noto_sans/NotoSans-Regular.ttf".as_ref(),
                                        &["fonts/noto_sans/NotoSans-Bold.ttf".as_ref(),
                                        "fonts/noto_sans/NotoSans-Italic.ttf".as_ref(),
                                        "fonts/noto_sans/NotoSans-BoldItalic.ttf".as_ref()]).unwrap())
                .expect("Could not create text renderer")
        };
        */

        FpsRenderer {
            show_fps: false,
            fps_string: String::with_capacity(12), // enough space to display "fps: xxx.yy"
            text_renderer: gfx_text::new(factory).build()?,
        }
    }

    fn render<C, T>(
        &mut self,
        dt_s: f32,
        encoder: &mut Encoder<R, C>,
        target: &RenderTargetView<R, T>)
    where
        C: CommandBuffer<R>,
        T: RenderFormat,
    {
        if self.show_fps {
            use std::fmt::Write;
            self.fps_string.write_fmt(format_args!("fps: {:.*}", 2, 1.0 / dt_s)).unwrap();
            self.text_renderer.add(&self.fps_string, [10, 20], [0.65, 0.16, 0.16, 1.0]);
            self.text_renderer.draw(encoder, target).unwrap();
            self.fps_string.clear();
        }
    }
}

#[cfg(windows)]
struct FpsRenderer<R: Resources, F: Factory<R>> {
    pub show_fps: bool,
    _marker: ::std::marker::PhantomData<(R, F)>,
}

#[cfg(windows)]
impl<R: Resources, F: Factory<R>> FpsRenderer<R, F> {
    #[inline]
    fn new(_factory: F) -> Result<Self, !> {
        Ok(FpsRenderer {
            show_fps: false,
            _marker: ::std::marker::PhantomData,
        })
    }

    #[inline]
    fn render<C, T>(
        &mut self,
        _dt_s: f32,
        _encoder: &mut Encoder<R, C>,
        _target: &RenderTargetView<R, T>)
    where
        C: CommandBuffer<R>,
        T: RenderFormat,
    {
    }
}
