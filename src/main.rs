extern crate angular;
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate num;
extern crate nalgebra as na;
#[macro_use]
extern crate scopeguard;
extern crate time;
extern crate wavefront_obj;

use angular::{Angle, Degrees};
use gfx::Device;
use gfx::traits::FactoryExt;
use na::{Isometry3, Perspective3, Point3, Rotation3, ToHomogeneous, Vector3};
use num::Zero;
use time::{Duration, PreciseTime};
use std::fs::File;
use std::io::Read;
use std::time::Duration as StdDuration;
use wavefront_obj::obj;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "position",
        col: [f32; 3] = "color",
    }

    constant Locals {
        transform: [[f32; 4]; 4] = "mvp_transform",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        locals: gfx::ConstantBuffer<Locals> = "locals",
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
    }
}

impl<'a> From<&'a obj::Vertex> for Vertex {
    fn from(v: &'a obj::Vertex) -> Self {
        Vertex {
            pos: [v.x as f32, v.y as f32, v.z as f32],
            col: [0.3f32, 0.3, 0.3],
        }
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
    fov: Angle<f32>
}

impl Input {
    fn new() -> Self {
        Input {
            position: Point3 { x: 0.0, y: 0.0, z: 10.0 },
            horizontal_angle: Angle::full(),
            vertical_angle: Angle::zero(),
            fov: Angle::eighth()
        }
    }
}

const SPEED: f32 = 19.0;
const MOUSE_SPEED: f32 = 7.0;
const DEFAULT_WIN_SIZE: (i32, i32) = (1024, 768);

fn main() {
    let builder = glutin::WindowBuilder::new()
        .with_title("Gfx Example")
        .with_dimensions(DEFAULT_WIN_SIZE.0 as u32, DEFAULT_WIN_SIZE.1 as u32)
        .with_decorations(false)
        .with_vsync();

    let (window, mut device, mut factory, main_color, _) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(builder);

    window.set_cursor_state(glutin::CursorState::Hide).expect("Could not set cursor state");
    window.set_cursor_state(glutin::CursorState::Grab).expect("Could not set cursor state");
    window.center_cursor().expect("Could not set cursor position");

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let program = factory.link_program(VERT_SRC, FRAG_SRC).unwrap();
    let pso = factory.create_pipeline_from_program(&program,
                                                   gfx::Primitive::TriangleList,
                                                   gfx::state::Rasterizer::new_fill().with_cull_back(),
                                                   pipe::new()).unwrap();

    let (verts, inds) = load_obj();
    let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&verts[..], &inds[..]);
    let data = pipe::Data {
        vbuf: vertex_buffer,
        locals: factory.create_constant_buffer(1),
        out: main_color
    };

    let mut rot = Rotation3::new(na::zero());
    let mut iput = Input::new();
    let mut projection = Perspective3::new(window.aspect(), iput.fov.in_radians(), 0.1, 100.0);
    let mut last = PreciseTime::now();
    let mut is_paused = false;

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
            z: iput.vertical_angle.cos() * iput.horizontal_angle.cos()
        };

        let right = Vector3 {
            x: (iput.horizontal_angle - Angle::quarter()).sin(),
            y: 0.0,
            z: (iput.horizontal_angle - Angle::quarter()).cos()
        };

        // Hack to get around lack of resize event on MacOS
        // https://github.com/tomaka/winit/issues/39
        if cfg!(target_os = "macos") {
            static mut WINDOW_LAST_W: i32 = 0;
            static mut WINDOW_LAST_H: i32 = 0;
            let (w, h) = window.get_size_signed_or_default();
            unsafe {
                if w != WINDOW_LAST_W || h != WINDOW_LAST_H {
                    projection.set_aspect(window.aspect());
                    WINDOW_LAST_W = w;
                    WINDOW_LAST_H = h;
                }
            }
        }

        for e in window.poll_events() {
            use glutin::{ElementState, Event, MouseScrollDelta, VirtualKeyCode};
            match e {
                Event::Closed | Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => break 'main,
                #[cfg(not(target_os = "macos"))]
                Event::Resized(w, h) => {
                    projection.set_aspect(aspect(w, h));
                }
                Event::MouseMoved(x, y) => {
                    let (ww, wh) = window.get_size_signed_or_default();

                    iput.horizontal_angle += Degrees(MOUSE_SPEED * dt_s * (ww / 2 - x) as f32);
                    iput.vertical_angle -= Degrees(MOUSE_SPEED * dt_s * (wh / 2 - y ) as f32);

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
                        MouseScrollDelta::PixelDelta(_, y) => y
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

        rot = na::append_rotation(&rot, &Vector3::new(0.0, Degrees(25.0 * dt_s).in_radians(), 0.0));

        let view = {
            let up = na::cross(&right, &direction);
            Isometry3::look_at_lh(&iput.position,
                                  &(iput.position.to_vector() + direction).to_point(),
                                  &up)
        };

        encoder.clear(&data.out, CLEAR_COLOR);

        let mvp = projection.to_matrix() * view.to_homogeneous() * rot.to_homogeneous();
        encoder.update_constant_buffer(&data.locals,
                                       &Locals {
                                           transform: *(mvp).as_ref()
                                       });

        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

fn load_obj() -> (Vec<Vertex>, Vec<u16>) {
    let mut obj_string  = String::new();
    let mut obj_file = File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/data/mesh/suzanne.obj"))
        .expect("Could not open suzanne.obj");
    obj_file.read_to_string(&mut obj_string).expect("Could not read suzanne.obj");
    drop(obj_file);

    let obj = obj::parse(obj_string).expect("Could not parse suzanne.obj");

    let mut vertices = Vec::new();
    let object = obj.objects.get(0).expect("No objects");
    for v in &object.vertices {
        vertices.push(v.into());
    }

    let mut indices = Vec::new();
    for g in &object.geometry {
        use wavefront_obj::obj::Primitive;
        for s in &g.shapes {
            match s.primitive {
                Primitive::Point(_) => unimplemented!(),
                Primitive::Line(..) => unimplemented!(),
                Primitive::Triangle(i0, i1, i2) => {
                    let (vi0, _, _) = i0;
                    let (vi1, _, _) = i1;
                    let (vi2, _, _) = i2;
                    indices.push(vi0 as u16);
                    indices.push(vi1 as u16);
                    indices.push(vi2 as u16);
                }
            }
        }
    }

    (vertices, indices)
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
