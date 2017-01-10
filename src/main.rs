#![feature(duration_checked_ops)]

extern crate angular;
#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate nalgebra as na;

use angular::Angle;
use gfx::{Device, Factory};
use gfx::traits::FactoryExt;
use na::{Isometry3, Perspective3, Point3, Rotation3, ToHomogeneous, Vector3};
use std::time::{Duration, Instant};

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
        transform: gfx::Global<[[f32; 4]; 4]> = "mvp_transform",
        locals: gfx::ConstantBuffer<Locals> = "locals",
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
    }
}

const VERT_SRC: &'static str = r#"
#version 150 core

in vec3 position;
in vec3 color;
out vec4 v_color;

uniform mat4 mvp_transform;

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

fn main() {
    let builder = glutin::WindowBuilder::new()
        .with_title("Gfx Example")
        .with_dimensions(1024, 768)
        .with_decorations(false)
        .with_vsync();

    let (mut window, mut device, mut factory, main_color, _) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(builder);

    window.set_window_resize_callback(Some(resize_cbk));

    let view_projection = {
        let aspect = window.get_inner_size_pixels().map(|(w, h)| w as f32 / h as f32).unwrap_or(1.0f32);
        let p = Perspective3::new(aspect, Angle::eighth().in_radians(), 0.1, 100.0).to_matrix();
        let v = Isometry3::look_at_rh(&Point3::new(4.0f32, 3.0, 3.0), &na::origin(), &Vector3::y())
            .to_homogeneous();
        p * v
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

    let mut rot = Rotation3::new(na::zero());
    'main: loop {
        let start = Instant::now();

        rot = na::append_rotation(&rot, &Vector3::new(0.0, Angle::Degrees(0.5).in_radians(), 0.0));
        let model = rot.to_homogeneous();

        let data = pipe::Data {
            vbuf: vertex_buffer.clone(),
            transform: *(view_projection * model).as_ref(),
            locals: factory.create_constant_buffer(1),
            out: main_color.clone()
        };

        for e in window.poll_events() {
            match e {
                glutin::Event::Closed => break 'main,
                _ => (),
            }
        }

        encoder.clear(&data.out, CLEAR_COLOR);
        encoder.draw(&slice, &pso, &data);
        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        let end = Instant::now();
        let dt = end.duration_since(start);

        let standard_fps = Duration::from_millis(16);
        let dt = standard_fps.checked_sub(dt).unwrap_or(standard_fps);

        std::thread::sleep(dt);
    }
}

fn resize_cbk(w: u32, h: u32){
}
