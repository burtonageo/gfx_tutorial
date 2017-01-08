#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;

use gfx::Device;
use gfx::traits::FactoryExt;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "position",
        col: [f32; 3] = "color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        out: gfx::RenderTarget<gfx::format::Rgba8> = "Target0",
    }
}

const VERT_SRC: &'static str = r#"
#version 150 core

in vec2 position;
in vec3 color;
out vec4 v_color;

void main() {
    v_color = vec4(color, 1.0);
    gl_Position = vec4(position, 0.0, 1.0);
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

const TRI: [Vertex; 3] = [
    Vertex { pos: [-0.5, -0.5], col: [1.0, 0.0, 0.0] },
    Vertex { pos: [ 0.5, -0.5], col: [0.0, 1.0, 0.0] },
    Vertex { pos: [ 0.0,  0.5], col: [0.0, 0.0, 1.0] }
];

const CLEAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 1.0];

fn main() {
    let builder = glutin::WindowBuilder::new()
        .with_title("Cube example")
        .with_dimensions(1024, 768)
        .with_vsync();

    let (window, mut device, mut factory, main_color, _) =
        gfx_window_glutin::init::<gfx::format::Rgba8, gfx::format::DepthStencil>(builder);

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let pso = factory.create_pipeline_simple(VERT_SRC.as_bytes(), FRAG_SRC.as_bytes(), pipe::new()).unwrap();
    let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&TRI, ());
    let data = pipe::Data {
        vbuf: vertex_buffer,
        out: main_color
    };

    'main: loop {
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
    }
}
