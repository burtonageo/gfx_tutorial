#![warn(missing_debug_implementations)]

extern crate alga;
extern crate apply;
extern crate ang;
extern crate find_folder;
#[macro_use]
extern crate gfx;
extern crate gfx_glyph;
extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate num;
extern crate nalgebra as na;
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
mod model;
mod util;

use ang::{Angle, Degrees};
use apply::Apply;
use gfx::{CommandBuffer, Device, Encoder, Resources, UpdateError};
use gfx_glyph::{GlyphBrushBuilder, Scale, Section};
use model::Model;
use na::{Isometry3, Matrix4, Perspective3, Point3, Point, UnitQuaternion, Vector3};
use num::{cast, NumCast, Zero};
use platform::{ContextBuilder, FactoryExt as PlFactoryExt, WindowExt as PlatformWindow};
use std::fs::File;
use std::io::Read;
use std::ops::{Div, Neg};
use std::time::Duration as StdDuration;
use time::{Duration, PreciseTime};

gfx_defines! {
    #[derive(Default)]
    vertex Vertex {
        pos: [f32; 3] = "position",
        uv: [f32; 2] = "tex_coord",
        normal: [f32; 3] = "normal",
    }

    #[derive(Default)]
    constant ShaderLight {
        col: [f32; 4] = "color",
        pos: [f32; 3] = "position",
        power: f32 = "power",
    }

    #[derive(Default)]
    constant VertLocals {
        projection: [[f32; 4]; 4] = "projection_matrix",
        model: [[f32; 4]; 4] = "model_matrix",
        view: [[f32; 4]; 4] = "view_matrix",
    }

    #[derive(Default)]
    constant SharedLocals {
        num_lights: u32 = "num_lights",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        vert_locals: gfx::ConstantBuffer<VertLocals> = "vert_locals",
        shared_locals: gfx::ConstantBuffer<SharedLocals> = "shared_locals",
        main_texture: gfx::TextureSampler<[f32; 4]> = "color_texture",
        lights: gfx::ConstantBuffer<ShaderLight> = "lights_array",
        out: gfx::RenderTarget<ColorFormat> = "Target0",
        main_depth: gfx::DepthTarget<DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;

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

// #[allow(dead_code)]
#[derive(Debug)]
struct Camera {
    perspective: Perspective3<f32>,
    view: Isometry3<f32>,
}

#[allow(dead_code)]
impl Camera {
    #[inline]
    fn matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (
            self.perspective.to_homogeneous(),
            self.view.to_homogeneous(),
        )
    }
}

const SPEED: f32 = 7.0;
const MOUSE_SPEED: f32 = 4.0;

#[derive(Clone, Debug, PartialEq)]
struct Light {
    position: Point3<f32>,
    color: [f32; 4],
    power: f32,
}

impl Light {
    #[inline]
    fn new(position: Point3<f32>, color: [f32; 4], power: f32) -> Self {
        Light {
            position,
            color,
            power,
        }
    }
}

impl Default for Light {
    #[inline]
    fn default() -> Self {
        Light::new(na::origin(), [na::zero(); 4], na::zero())
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

#[derive(Debug, Default)]
struct Scene<R: Resources> {
    lights: Vec<ShaderLight>,
    models: Vec<Model<R>>,
}

impl<R: Resources> Scene<R> {
    fn new<L: Into<ShaderLight>>(lights: Vec<L>, models: Vec<Model<R>>) -> Self {
        let lights = lights.into_iter().map(Into::into).collect();
        Scene { lights, models }
    }

    fn update_views<W: PlatformWindow<R>>(&mut self, window: &W) {
        for model in &mut self.models {
            model.update_views(window)
        }
    }

    fn render<C: CommandBuffer<R>>(
        &self,
        encoder: &mut Encoder<R, C>,
        view_matrix: Matrix4<f32>,
        projection_matrix: Matrix4<f32>,
    ) -> Result<(), UpdateError<usize>> {
        for model in &self.models {
            model.update_matrices(encoder, &view_matrix, &projection_matrix);
            model.update_lights(encoder, &self.lights)?;
            model.encode(encoder);
        }
        Ok(())
    }
}

fn main() {
    let mut events_loop = winit::EventsLoop::new();
    let builder = {
        let primary_monitor = events_loop.get_primary_monitor();
        let (win_w, win_h) = primary_monitor.get_dimensions();
        winit::WindowBuilder::new()
            .with_dimensions(win_w, win_h)
            // .with_fullscreen(primary_monitor)
            .with_title("Gfx Example")
    };

    let (backend, window, mut device, mut factory, main_color, main_depth) =
        platform::launch_gl::<ColorFormat, gfx::format::DepthStencil>(
            builder,
            &events_loop,
            ContextBuilder::new().with_vsync_enabled(true),
        ).expect("Could not create window or graphics device");

    window.hide_and_grab_cursor().expect(
        "Could not set cursor state",
    );
    window.center_cursor().expect(
        "Could not set cursor position",
    );

    let mut encoder = factory.create_encoder();

    let fonts = [
        "NotoSans-Bold.ttf",
        "NotoSans-BoldItalic.ttf",
        "NotoSans-Italic.ttf",
        "NotoSans-Regular.ttf",
    ];
    let mut glyph_brush = fonts
        .iter()
        .map(|p| {
            util::get_assets_folder()
                .unwrap()
                .to_path_buf()
                .join("fonts")
                .join("noto_sans")
                .join(p)
        })
        .map(|p| {
            let mut bytes = Vec::new();
            File::open(p)
                .and_then(|mut f| f.read_to_end(&mut bytes))
                .unwrap();
            bytes
        })
        .collect::<Vec<_>>()
        .apply(GlyphBrushBuilder::using_fonts_bytes)
        .build(factory.clone());

    let mut scene = {
        let models = {
            let mut monkey_model = Model::load(
                &mut factory,
                &backend,
                main_color.clone(),
                main_depth.clone(),
                "suzanne",
                "img/checker.png",
            ).expect("Could not load model");

            let mut cube_model = Model::load(
                &mut factory,
                &backend,
                main_color.clone(),
                main_depth.clone(),
                "cube",
                "img/checker.png",
            ).expect("Could not load model");

            monkey_model.similarity.isometry.translation.vector[1] += 2.0f32;
            cube_model.similarity.isometry.translation.vector[1] -= 2.0f32;

            vec![monkey_model, cube_model]
        };

        let lights: Vec<ShaderLight> = {
            let l1 = Light::new(Point3::new(0.0, 3.0, -2.0), [1.0, 0.0, 0.0, 1.0], 300.0);
            let l2 = Light::new(Point3::new(0.0, 1.6, 0.0), [1.0, 0.0, 0.0, 1.0], 400.0);
            let l3 = Light::new(Point3::new(1.5, -3.0, 0.0), [1.0, 0.0, 1.0, 0.3], 300.0);
            let l4 = Light::new(Point3::new(0.0, -1.8, 0.0), [1.0, 0.0, 1.0, 1.0], 400.0);

            vec![l1, l2, l3, l4].into_iter().map(Into::into).collect()
        };

        Scene::new(lights, models)
    };

    let mut iput = Input::new();
    let mut projection = Perspective3::new(window.aspect(), iput.fov.in_radians(), 0.1, 100.0);
    let mut last = PreciseTime::now();
    let mut is_paused = false;

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

        let direction = Vector3::new(
            iput.vertical_angle.cos() * iput.horizontal_angle.sin(),
            iput.vertical_angle.sin(),
            iput.vertical_angle.cos() * iput.horizontal_angle.cos(),
        );

        let right = Vector3::new(
            (iput.horizontal_angle - Angle::quarter()).sin(),
            na::zero(),
            (iput.horizontal_angle - Angle::quarter()).cos(),
        );

        // Hack to get around lack of resize event on MacOS
        // https://github.com/tomaka/winit/issues/39
        if cfg!(target_os = "macos") {
            static mut WINDOW_LAST_W: i32 = 0;
            static mut WINDOW_LAST_H: i32 = 0;
            let (w, h) = window.windowext_get_inner_size();
            unsafe {
                if w != WINDOW_LAST_W || h != WINDOW_LAST_H {
                    scene.update_views(&window);
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
                        WindowEvent::KeyboardInput {
                            input: KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        .. } => {
                            is_running = false;
                        }
                        #[cfg(not(target_os = "macos"))]
                        WindowEvent::Resized(..) => {
                            scene.update_views(&window);
                            projection.set_aspect(window.aspect());
                        }
                        WindowEvent::CursorMoved { position: (x, y), .. } => {
                            let (ww, wh) = window.windowext_get_inner_size::<i32>();
                            let hidpi = window.hidpi_factor() as f64;

                            iput.horizontal_angle += Degrees(
                                MOUSE_SPEED * dt_s *
                                    ((ww / 2) as f32 - (x / hidpi) as f32),
                            );
                            iput.vertical_angle -= Degrees(
                                MOUSE_SPEED * dt_s *
                                    ((wh / 2) as f32 - (y / hidpi) as f32),
                            );

                            iput.horizontal_angle = iput.horizontal_angle.normalized();

                            let threshold = Angle::quarter() - Degrees(1.0f32);

                            if iput.vertical_angle > threshold {
                                iput.vertical_angle = threshold;
                            }

                            if iput.vertical_angle < threshold.neg() {
                                iput.vertical_angle = threshold.neg();
                            }

                            window.center_cursor().expect(
                                "Could not set cursor position",
                            );
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            match input {
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Up),
                                    ..
                                } => {
                                    iput.position -= direction * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Down),
                                    ..
                                } => {
                                    iput.position += direction * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Left),
                                    ..
                                } => {
                                    iput.position += right * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Right),
                                    ..
                                } => {
                                    iput.position -= right * SPEED * dt_s;
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

        let rot = UnitQuaternion::from_euler_angles(0.0, Degrees(25.0 * dt_s).in_radians(), 0.0);
        for model in &mut scene.models {
            model.similarity.append_rotation_mut(&rot);
        }

        let view = {
            let up = right.cross(&direction);
            Isometry3::look_at_lh(
                &iput.position,
                &Point::from_coordinates(iput.position.coords + direction),
                &up,
            )
        };

        encoder.clear(&main_color, CLEAR_COLOR);
        encoder.clear_depth(&main_depth, 1.0);

        let view_mat = view.to_homogeneous();
        let projection_mat = projection.to_homogeneous();

        glyph_brush.queue(Section {
            text: "Hello, World!",
            screen_position: (5.0, 5.0),
            scale: Scale::uniform(32.0f32 * window.hidpi_factor()),
            color: [1.0, 1.0, 1.0, 1.0],
            z: 1.0,
            ..Default::default()
        });
        glyph_brush
            .draw_queued(&mut encoder, &main_color, &main_depth)
            .unwrap();

        scene
            .render(&mut encoder, view_mat, projection_mat)
            .expect("Could not render scene");

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

trait WindowExt {
    fn center_cursor(&self) -> Result<(), ()>;
    fn hide_and_grab_cursor(&self) -> Result<(), String>;
    fn windowext_get_inner_size<N: NumCast + Zero + Default>(&self) -> (N, N);
    fn aspect<N: Default + Div<Output = N> + NumCast + Zero>(&self) -> N {
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
            (
                cast(x).unwrap_or(Zero::zero()),
                cast(y).unwrap_or(Zero::zero()),
            )
        }

        self.get_inner_size().map(cast_pair).unwrap_or(
            Default::default(),
        )
    }
}
