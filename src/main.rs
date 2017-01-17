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

use angular::Angle;
use gfx::{Device, Factory};
use gfx::traits::FactoryExt;
use na::{Isometry3, Perspective3, Point3, Rotation3, ToHomogeneous, Vector3};
use num::Zero;
use time::{Duration, PreciseTime};
use std::time::Duration as StdDuration;

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

const VERT_SRC: &'static str = r#"
#version 150 core

in vec3 position;
in vec3 color;
out vec4 v_color;

layout (std140) uniform locals {
    mat4 mvp_transform;
};

void main() {
    v_color = vec4(color, 1.0);
    gl_Position = mvp_transform * vec4(position, 1.0);
    gl_ClipDistance[0] = 1.0;
}
"#;

const FRAG_SRC: &'static str = r#"
#version 150 core

in vec4 v_color;
out vec4 Target0;

void main() {
    Target0 = v_color;
}
"#;

const CUBE: &'static [Vertex] = &[
    Vertex { pos: [-1.0, -1.0, -1.0], col: [0.583, 0.771, 0.014] },
    Vertex { pos: [-1.0, -1.0,  1.0], col: [0.609, 0.115, 0.436] },
    Vertex { pos: [-1.0,  1.0,  1.0], col: [0.327, 0.483, 0.844] },
    Vertex { pos: [ 1.0,  1.0, -1.0], col: [0.822, 0.569, 0.201] },
    Vertex { pos: [-1.0, -1.0, -1.0], col: [0.435, 0.602, 0.223] },
    Vertex { pos: [-1.0,  1.0, -1.0], col: [0.310, 0.747, 0.185] },
    Vertex { pos: [ 1.0, -1.0,  1.0], col: [0.597, 0.770, 0.761] },
    Vertex { pos: [-1.0, -1.0, -1.0], col: [0.559, 0.436, 0.730] },
    Vertex { pos: [ 1.0, -1.0, -1.0], col: [0.359, 0.583, 0.152] },
    Vertex { pos: [ 1.0,  1.0, -1.0], col: [0.483, 0.596, 0.789] },
    Vertex { pos: [ 1.0, -1.0, -1.0], col: [0.559, 0.861, 0.639] },
    Vertex { pos: [-1.0, -1.0, -1.0], col: [0.195, 0.548, 0.859] },
    Vertex { pos: [-1.0, -1.0, -1.0], col: [0.014, 0.184, 0.576] },
    Vertex { pos: [-1.0,  1.0,  1.0], col: [0.771, 0.328, 0.970] },
    Vertex { pos: [-1.0,  1.0, -1.0], col: [0.406, 0.615, 0.116] },
    Vertex { pos: [ 1.0, -1.0,  1.0], col: [0.676, 0.977, 0.133] },
    Vertex { pos: [-1.0, -1.0,  1.0], col: [0.971, 0.572, 0.833] },
    Vertex { pos: [-1.0, -1.0, -1.0], col: [0.140, 0.616, 0.489] },
    Vertex { pos: [-1.0,  1.0,  1.0], col: [0.997, 0.513, 0.064] },
    Vertex { pos: [-1.0, -1.0,  1.0], col: [0.945, 0.719, 0.592] },
    Vertex { pos: [ 1.0, -1.0,  1.0], col: [0.543, 0.021, 0.978] },
    Vertex { pos: [ 1.0,  1.0,  1.0], col: [0.279, 0.317, 0.505] },
    Vertex { pos: [ 1.0, -1.0, -1.0], col: [0.167, 0.620, 0.077] },
    Vertex { pos: [ 1.0,  1.0, -1.0], col: [0.347, 0.857, 0.137] },
    Vertex { pos: [ 1.0, -1.0, -1.0], col: [0.055, 0.953, 0.042] },
    Vertex { pos: [ 1.0,  1.0,  1.0], col: [0.714, 0.505, 0.345] },
    Vertex { pos: [ 1.0, -1.0,  1.0], col: [0.783, 0.290, 0.734] },
    Vertex { pos: [ 1.0,  1.0,  1.0], col: [0.722, 0.645, 0.174] },
    Vertex { pos: [ 1.0,  1.0, -1.0], col: [0.302, 0.455, 0.848] },
    Vertex { pos: [-1.0,  1.0, -1.0], col: [0.225, 0.587, 0.040] },
    Vertex { pos: [ 1.0,  1.0,  1.0], col: [0.517, 0.713, 0.338] },
    Vertex { pos: [-1.0,  1.0, -1.0], col: [0.053, 0.959, 0.120] },
    Vertex { pos: [-1.0,  1.0,  1.0], col: [0.393, 0.621, 0.362] },
    Vertex { pos: [ 1.0,  1.0,  1.0], col: [0.673, 0.211, 0.457] },
    Vertex { pos: [-1.0,  1.0,  1.0], col: [0.820, 0.883, 0.371] },
    Vertex { pos: [ 1.0, -1.0,  1.0], col: [0.982, 0.099, 0.879] }
];

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
const DEFAULT_WIN_SIZE: (u32, u32) = (1024, 768);

fn main() {
    let builder = glutin::WindowBuilder::new()
        .with_title("Gfx Example")
        .with_dimensions(DEFAULT_WIN_SIZE.0 as u32, DEFAULT_WIN_SIZE.1 as u32)
        .with_decorations(false)
        .with_vsync();

    let (window, mut device, mut factory, main_color, _) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(builder);

    {
        window.set_cursor_state(glutin::CursorState::Hide).expect("Could not set cursor state");
        window.set_cursor_state(glutin::CursorState::Grab).expect("Could not set cursor state");
        center_cursor_to_window(&window).expect("Could not set cursor position");
    }

    let mut iput = Input::new();

    let mut projection = {
        let aspect = window.get_inner_size_pixels().map(|(w, h)| aspect(w, h)).unwrap_or(1.0f32);
        Perspective3::new(aspect, iput.fov.in_radians(), 0.1, 100.0)
    };

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let shaders = {
        let vert_shader = factory.create_shader_vertex(VERT_SRC.as_bytes()).unwrap();
        let frag_shader = factory.create_shader_pixel(FRAG_SRC.as_bytes()).unwrap();
        gfx::ShaderSet::Simple(vert_shader, frag_shader)
    };
    let pso = factory.create_pipeline_state(&shaders,
                                            gfx::Primitive::TriangleList,
                                            gfx::state::Rasterizer::new_fill().with_cull_back(),
                                            pipe::new()).unwrap();
    let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(CUBE, ());
    let data = pipe::Data {
        vbuf: vertex_buffer,
        locals: factory.create_constant_buffer(1),
        out: main_color
    };

    let mut rot = Rotation3::new(na::zero());
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

        for e in window.poll_events() {
            use glutin::{ElementState, Event, MouseScrollDelta, VirtualKeyCode};
            match e {
                Event::Closed | Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) => break 'main,
                Event::Resized(w, h) => {
                    // TODO(GAB): This doesn't work on MacOS
                    projection.set_aspect(aspect(w, h));
                }
                Event::MouseMoved(x, y) => {
                    let (ww, wh) = window.get_inner_size().unwrap_or(DEFAULT_WIN_SIZE);
                    let (ww, wh) = (ww as i32, wh as i32);

                    iput.horizontal_angle += Angle::Degrees(MOUSE_SPEED * dt_s * (ww / 2 - x) as f32);
                    iput.vertical_angle -= Angle::Degrees(MOUSE_SPEED * dt_s * (wh / 2 - y ) as f32);

                    iput.horizontal_angle = iput.horizontal_angle.normalized();
                    iput.vertical_angle = iput.vertical_angle.normalized();

                    center_cursor_to_window(&window).expect("Could not set cursor position");
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

        rot = na::append_rotation(&rot, &Vector3::new(0.0, Angle::Degrees(25.0 * dt_s).in_radians(), 0.0));

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

fn center_cursor_to_window(window: &glutin::Window) -> Result<(), ()> {
    let (ww, wh) = window.get_inner_size().unwrap_or(DEFAULT_WIN_SIZE);
    window.set_cursor_position(ww as i32 / 2, wh as i32 / 2)
}

fn aspect(w: u32, h: u32) -> f32 {
    w as f32 / h as f32
}
