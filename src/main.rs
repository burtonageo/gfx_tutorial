#![feature(conservative_impl_trait)]

extern crate alga;
extern crate ang;
extern crate find_folder;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_rusttype;
extern crate gfx_window_glutin;
extern crate glutin;
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

#[cfg(target_os = "macos")]
extern crate gfx_window_metal;
#[cfg(target_os = "macos")]
extern crate gfx_device_metal;

#[cfg(target_os = "windows")]
extern crate gfx_window_dxgi;
#[cfg(target_os = "windows")]
extern crate gfx_device_dx11;

mod load;
mod platform;
mod util;

use ang::{Angle, Degrees};
use gfx::{Bundle, Device, Factory, Resources};
use gfx::format::Rgba8;
use gfx::texture::{AaMode, Kind};
use gfx::traits::FactoryExt;
use load::load_obj;
use na::{Isometry3, Perspective3, Point3, PointBase, Rotation3, Vector3};
use num::Zero;
use platform::{FactoryExt as PlFactoryExt, Window, WinitWindowExt as PlatformWindow};
use std::env::args;
use std::ops::Neg;
use std::time::Duration as StdDuration;
use time::{Duration, PreciseTime};
use util::get_assets_folder;

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
        main_depth: gfx::DepthTarget<gfx::format::Depth32F> = gfx::preset::depth::LESS_EQUAL_WRITE,
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
const DEFAULT_WIN_SIZE: (i32, i32) = (1024, 768);

#[derive(Clone, Debug, PartialEq)]
struct Light {
    position: Point3<f32>,
    color: [f32; 4],
    power: f32,
}

impl Default for Light {
    fn default() -> Self {
        Light {
            position: na::origin(),
            color: [na::zero(); 4],
            power: na::zero(),
        }
    }
}

impl From<Light> for ShaderLight {
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
    let builder = winit::WindowBuilder::new()
        .with_title("Gfx Example")
        .with_dimensions(DEFAULT_WIN_SIZE.0 as u32, DEFAULT_WIN_SIZE.1 as u32)
        .with_decorations(false);

    let (backend, window, mut device, mut factory, main_color, main_depth) =
        platform::launch_native::<Rgba8, gfx::format::Depth32F>(builder)
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

    let (verts, inds) = load_obj(&args().nth(1).unwrap_or("suzanne".into()));
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

    let text_renderer = {
        const POS_TOLERANCE: f32 = 0.1;
        const SCALE_TOLERANCE: f32 = 0.1;
        let (w, h) = window.as_winit_window().get_inner_size().unwrap_or((0u32, 0u32));
        gfx_rusttype::TextRenderer::new(&mut factory, w as u16, h as u16, POS_TOLERANCE, SCALE_TOLERANCE).unwrap()
    };

    // let mut text = gfx_text::new(factory).build().expect("Could not create text renderer");
    let mut bundle = Bundle::new(slice, pso, data);

    let mut rot = Rotation3::identity();
    let mut iput = Input::new();
    let mut projection = Perspective3::new(window.aspect(), iput.fov.in_radians(), 0.1, 100.0);
    let mut last = PreciseTime::now();
    let mut is_paused = false;

    let mut show_fps = false;
    let mut fps_string = String::with_capacity(12); // enough space to display "fps: xxx.yy"

    'main: loop {
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
            let (w, h) = window.get_size_signed_or_default();
            unsafe {
                if w != WINDOW_LAST_W || h != WINDOW_LAST_H {
                    window.update_views(&mut bundle.data.out, &mut bundle.data.main_depth);
                    projection.set_aspect(window.aspect());
                    WINDOW_LAST_W = w;
                    WINDOW_LAST_H = h;
                }
            }
        }

        for e in window.as_winit_window().poll_events() {
            use winit::{ElementState, Event, VirtualKeyCode};
            match e {
                Event::Closed |
                Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => break 'main,
                #[cfg(not(target_os = "macos"))]
                Event::Resized(w, h) => {
                    window.update_views(&mut bundle.data.out, &mut bundle.data.main_depth);
                    projection.set_aspect(window.aspect());
                }
                Event::MouseMoved(x, y) => {
                    let (ww, wh) = window.get_size_signed_or_default();
                    let hidpi = window.as_winit_window().hidpi_factor() as i32;

                    iput.horizontal_angle += Degrees(MOUSE_SPEED * dt_s * (ww / 2 - (x / hidpi)) as f32);
                    iput.vertical_angle -= Degrees(MOUSE_SPEED * dt_s * (wh / 2 - (y / hidpi)) as f32);

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
                Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::Up)) => {
                    iput.position -= direction * SPEED * dt_s;
                }
                Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::Down)) => {
                    iput.position += direction * SPEED * dt_s;
                }
                Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::Left)) => {
                    iput.position += right * SPEED * dt_s;
                }
                Event::KeyboardInput(ElementState::Pressed, _, Some(VirtualKeyCode::Right)) => {
                    iput.position -= right * SPEED * dt_s;
                }
                Event::KeyboardInput(ElementState::Released, _, Some(VirtualKeyCode::Space)) => {
                    show_fps = !show_fps;
                }
                Event::Focused(gained) => {
                    is_paused = !gained;
                    continue;
                }
                _ => (),
            }
        }

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

        if show_fps {
            use std::fmt::Write;
            fps_string.write_fmt(format_args!("fps: {:.*}", 2, 1.0 / dt_s)).unwrap();
            text_renderer.encode(&mut encoder);
            println!("{}", fps_string);
            fps_string.clear();
        }

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

trait WindowExt<R: Resources>: PlatformWindow<R> {
    fn center_cursor(&self) -> Result<(), ()>;
    fn hide_and_grab_cursor(&self) -> Result<(), String>;
    fn get_size_signed_or_default(&self) -> (i32, i32);
    fn aspect(&self) -> f32 {
        let (w, h) = self.get_size_signed_or_default();
        w as f32 / h as f32
    }
}

impl<R: Resources, W: PlatformWindow<R>> WindowExt<R> for W {
    fn center_cursor(&self) -> Result<(), ()> {
        let (ww, wh) = self.get_size_signed_or_default();
        self.as_winit_window().set_cursor_position(ww as i32 / 2, wh as i32 / 2)
    }

    fn hide_and_grab_cursor(&self) -> Result<(), String> {
        self.as_winit_window().set_cursor_state(winit::CursorState::Hide)?;
        self.as_winit_window().set_cursor_state(winit::CursorState::Grab)
    }

    fn get_size_signed_or_default(&self) -> (i32, i32) {
        fn u32pair_toi32pair((x, y): (u32, u32)) -> (i32, i32) {
            (x as i32, y as i32)
        }

        self.as_winit_window()
            .get_inner_size()
            .map(u32pair_toi32pair)
            .unwrap_or(DEFAULT_WIN_SIZE)
    }
}
