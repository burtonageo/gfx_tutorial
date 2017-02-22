extern crate angular;
extern crate find_folder;
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate num;
extern crate nalgebra as na;
#[macro_use]
extern crate scopeguard;
extern crate time;
extern crate wavefront_obj;

mod model_load;
mod util;

use angular::{Angle, Degrees};
use gfx::{Device, Factory};
use gfx::texture::{AaMode, FilterMethod, Kind, SamplerInfo, WrapMode};
use gfx::traits::FactoryExt;
use image::GenericImage;
use model_load::load_obj;
use na::{Isometry3, Perspective3, Point3, Rotation3, ToHomogeneous, Vector3};
use num::Zero;
use std::env::args;
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

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        vert_locals: gfx::ConstantBuffer<VertLocals> = "vert_locals",
        shared_locals: gfx::ConstantBuffer<SharedLocals> = "shared_locals",
        main_texture: gfx::TextureSampler<[f32; 4]> = "color_texture",
        lights: gfx::ConstantBuffer<ShaderLight> = "lights_array",
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
        main_depth: gfx::DepthTarget<gfx::format::Depth> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

const VERT_SRC: &'static [u8] = include_bytes!("../data/shader/standard.vs");
const FRAG_SRC: &'static [u8] = include_bytes!("../data/shader/standard.fs");
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
            position: Point3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            },
            horizontal_angle: Angle::full(),
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
        let Point3 { x, y, z } = l.position;
        ShaderLight {
            pos: [x, y, z],
            col: l.color,
            power: l.power,
        }
    }
}

const MAX_LIGHTS: usize = 10;

fn main() {
    let builder = glutin::WindowBuilder::new()
        .with_title("Gfx Example")
        .with_dimensions(DEFAULT_WIN_SIZE.0 as u32, DEFAULT_WIN_SIZE.1 as u32)
        .with_decorations(false)
        .with_vsync();

    let (window, mut device, mut factory, main_color, main_depth) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::Depth>(builder);

    window.set_cursor_state(glutin::CursorState::Hide).expect("Could not set cursor state");
    window.set_cursor_state(glutin::CursorState::Grab).expect("Could not set cursor state");
    window.center_cursor().expect("Could not set cursor position");

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let program = factory.link_program(VERT_SRC, FRAG_SRC).unwrap();
    let pso = factory.create_pipeline_from_program(&program,
                                      gfx::Primitive::TriangleList,
                                      gfx::state::Rasterizer::new_fill().with_cull_back(),
                                      pipe::new())
        .expect("Could not create pso");

    let mut img_path = get_assets_folder().unwrap().to_path_buf();
    img_path.push("img/checker.png");
    let img = image::open(img_path).expect("Could not open image");
    let (iw, ih) = img.dimensions();
    let pixels = img.raw_pixels();
    let (_, srv) =
        factory.create_texture_immutable_u8::<[f32; 4]>(Kind::D2(iw as u16,
                                                              ih as u16,
                                                              AaMode::Single),
                                                     &[&pixels[..]])
            .expect("Could not create texture");

    let sampler = factory.create_sampler(SamplerInfo::new(FilterMethod::Scale, WrapMode::Clamp));

    let (verts, inds) = load_obj(&args().nth(1).unwrap_or("suzanne".into()));
    let (vertex_buffer, vslice) = factory.create_vertex_buffer_with_slice(&verts[..], &inds[..]);
    let mut data = pipe::Data {
        vbuf: vertex_buffer,
        vert_locals: factory.create_constant_buffer(1),
        shared_locals: factory.create_constant_buffer(1),
        lights: factory.create_constant_buffer(MAX_LIGHTS),
        main_texture: (srv, sampler),
        out: main_color,
        main_depth: main_depth,
    };

    let mut rot = Rotation3::new(na::zero());
    let mut iput = Input::new();
    let mut projection = Perspective3::new(window.aspect(), iput.fov.in_radians(), 0.1, 100.0);
    let mut last = PreciseTime::now();
    let mut is_paused = false;

    // Initial sleep to ensure that everything is initialised before
    // events are processed.
    std::thread::sleep(StdDuration::from_millis(30));

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

        let direction = Vector3 {
            x: iput.vertical_angle.cos() * iput.horizontal_angle.sin(),
            y: iput.vertical_angle.sin(),
            z: iput.vertical_angle.cos() * iput.horizontal_angle.cos(),
        };

        let right = Vector3 {
            x: (iput.horizontal_angle - Angle::quarter()).sin(),
            y: na::zero(),
            z: (iput.horizontal_angle - Angle::quarter()).cos(),
        };

        // Hack to get around lack of resize event on MacOS
        // https://github.com/tomaka/winit/issues/39
        if cfg!(target_os = "macos") {
            static mut WINDOW_LAST_W: i32 = 0;
            static mut WINDOW_LAST_H: i32 = 0;
            let (w, h) = window.get_size_signed_or_default();
            unsafe {
                if w != WINDOW_LAST_W || h != WINDOW_LAST_H {
                    gfx_window_glutin::update_views(&window, &mut data.out, &mut data.main_depth);
                    projection.set_aspect(window.aspect());
                    WINDOW_LAST_W = w;
                    WINDOW_LAST_H = h;
                }
            }
        }

        for e in window.poll_events() {
            use glutin::{ElementState, Event, MouseScrollDelta, VirtualKeyCode};
            match e {
                Event::Closed |
                Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => break 'main,
                #[cfg(not(target_os = "macos"))]
                Event::Resized(w, h) => {
                    projection.set_aspect(aspect(w, h));
                    gfx_window_glutin::update_views(&window, &mut data.out, &mut data.main_depth)
                }
                Event::MouseMoved(x, y) => {
                    let (ww, wh) = window.get_size_signed_or_default();

                    iput.horizontal_angle += Degrees(MOUSE_SPEED * dt_s * (ww / 2 - x) as f32);
                    iput.vertical_angle -= Degrees(MOUSE_SPEED * dt_s * (wh / 2 - y) as f32);

                    iput.horizontal_angle = iput.horizontal_angle.normalized();
                    iput.vertical_angle = iput.vertical_angle.normalized();

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
                Event::MouseWheel(delta, _) => {
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(_, y) => y,
                    };
                    iput.fov = (iput.fov + Angle::Degrees(dy / 5.0)).normalized();
                    projection.set_fovy(iput.fov.in_radians());
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

        rot = na::append_rotation(&rot,
                                  &Vector3::new(0.0, Degrees(25.0 * dt_s).in_radians(), 0.0));

        let view = {
            let up = na::cross(&right, &direction);
            Isometry3::look_at_lh(&iput.position,
                                  &(iput.position.to_vector() + direction).to_point(),
                                  &up)
        };

        encoder.clear(&data.out, CLEAR_COLOR);
        encoder.clear_depth(&data.main_depth, 1.0);

        let view_mat = view.to_homogeneous();
        let model_mat = rot.to_homogeneous();
        let mvp = projection.to_matrix() * view_mat * model_mat;
        let light = Light {
                position: Point3::new(0.0, 0.0001, 4.0),
                color: [0.1, 0.3, 0.8, 0.8],
                power: 200.0,
            }
            .into();
        encoder.update_constant_buffer(&data.vert_locals,
                                       &VertLocals {
                                           transform: *(mvp).as_ref(),
                                           model: *(model_mat).as_ref(),
                                           view: *(view_mat).as_ref(),
                                       });
        encoder.update_constant_buffer(&data.shared_locals, &SharedLocals { num_lights: 1 });
        encoder.update_buffer(&data.lights, &[light], 0).expect("Could not update buffer");

        encoder.draw(&vslice, &pso, &data);
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

trait WindowExt {
    fn center_cursor(&self) -> Result<(), ()>;
    fn get_size_signed_or_default(&self) -> (i32, i32);
    fn aspect(&self) -> f32 {
        let (w, h) = self.get_size_signed_or_default();
        w as f32 / h as f32
    }
}

impl WindowExt for glutin::Window {
    fn center_cursor(&self) -> Result<(), ()> {
        let (ww, wh) = self.get_size_signed_or_default();
        self.set_cursor_position(ww as i32 / 2, wh as i32 / 2)
    }

    fn get_size_signed_or_default(&self) -> (i32, i32) {
        fn u32pair_toi32pair((x, y): (u32, u32)) -> (i32, i32) {
            (x as i32, y as i32)
        }

        self.get_inner_size().map(u32pair_toi32pair).unwrap_or(DEFAULT_WIN_SIZE)
    }
}
