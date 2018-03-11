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

mod controllers;
mod graphics;
mod util;

use ang::Degrees;
use apply::Apply;
use controllers::camera_controller::CameraController;
use gfx::{CommandBuffer, Device, Encoder, Resources, UpdateError};
use gfx_glyph::{FontId, GlyphBrushBuilder, Layout, BuiltInLineBreaker, Scale, Section};
use graphics::camera::{Camera, CameraMatrices};
use graphics::fps_counter::FpsCounter;
use graphics::model::Model;
use graphics::platform::{self, ContextBuilder, FactoryExt as PlFactoryExt, WindowExt as PlatformWindow};
use na::{Point3, UnitQuaternion};
use num::{cast, NumCast, Zero};
use std::fs::File;
use std::io::Read;
use std::ops::Div;
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

    fn render<CBuf: CommandBuffer<R>>(
        &self,
        encoder: &mut Encoder<R, CBuf>,
        camera: &CameraController,
    ) -> Result<(), UpdateError<usize>> {
        let CameraMatrices { view, projection } = camera.matrices();
        for model in &self.models {
            model.update_matrices(encoder, &view, &projection);
            model.update_lights(encoder, &self.lights)?;
            model.encode(encoder);
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Styling {
    pub screen_position: (f32, f32),
    pub bounds: (f32, f32),
    pub scale: Scale,
    pub color: [f32; 4],
    pub z: f32,
    pub layout: Layout<BuiltInLineBreaker>,
    pub font_id: FontId,
}

impl Default for Styling {
    #[inline]
    fn default() -> Self {
        Styling {
            screen_position: Default::default(),
            bounds: Default::default(),
            scale: Scale::uniform(1.0),
            color: Default::default(),
            z: Default::default(),
            layout: Default::default(),
            font_id: Default::default(),
        }
    }
}

#[allow(dead_code)]
impl Styling {
    fn with_scale(self, scale: f32, window: Option<&winit::Window>) -> Self {
        let hidpi_scale = window.map(|w| w.hidpi_factor()).unwrap_or(1.0);
        Styling {
            scale: Scale::uniform(scale * hidpi_scale),
            ..self
        }
    }

    #[inline]
    fn to_section<'a>(&self, text: &'a str) -> Section<'a> {
        Section {
            text,
            screen_position: self.screen_position,
            bounds: self.bounds,
            scale: self.scale,
            color: self.color,
            z: self.z,
            layout: self.layout,
            font_id: self.font_id,
        }
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

    const FONTS: &[&str] = &[
        "NotoSans-Bold.ttf",
        "NotoSans-BoldItalic.ttf",
        "NotoSans-Italic.ttf",
        "NotoSans-Regular.ttf",
    ];
    let mut glyph_brush = FONTS
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

    let mut cam_controller = CameraController::new(window.window(), MOUSE_SPEED);
    let mut last = PreciseTime::now();
    let mut is_paused = false;

    let mut is_running = true;
    let mut fps = FpsCounter::new();

    while is_running {
        let current = PreciseTime::now();
        let dt = last.to(current);
        let dt_s = dt.as_seconds();
        fps.update_fps(dt_s);
        last = current;

        defer!({
            let sleep_time = Duration::milliseconds(12)
                .checked_sub(&dt)
                .unwrap_or(Duration::zero())
                .to_std()
                .unwrap_or(StdDuration::from_millis(0));
            std::thread::sleep(sleep_time);
        });

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
                        WindowEvent::Resized(..) => {
                            scene.update_views(&window);
                            cam_controller.on_resize(window.window());
                        }
                        WindowEvent::CursorMoved { position: (x, y), .. } => {
                            let (ww, wh) = window.windowext_get_inner_size::<i32>();
                            let hidpi = window.hidpi_factor() as f64;

                            let h = Degrees(
                                MOUSE_SPEED * dt_s *
                                    ((ww / 2) as f32 - (x / hidpi) as f32),
                            );
                            let v = Degrees(
                                MOUSE_SPEED * dt_s *
                                    ((wh / 2) as f32 - (y / hidpi) as f32),
                            );

                            cam_controller.rotate_view_angles_by(h, v);

                            window.center_cursor().expect(
                                "Could not set cursor position",
                            );
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            let right = cam_controller.right();
                            let direction = cam_controller.direction();
                            match input {
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Up),
                                    ..
                                } => {
                                    cam_controller.position -= direction * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Down),
                                    ..
                                } => {
                                    cam_controller.position += direction * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Left),
                                    ..
                                } => {
                                    cam_controller.position += right * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Right),
                                    ..
                                } => {
                                    cam_controller.position -= right * SPEED * dt_s;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Space),
                                    ..
                                } => {
                                    fps.toggle_show_fps();
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

        encoder.clear(&main_color, CLEAR_COLOR);
        encoder.clear_depth(&main_depth, 1.0);

        let styling = Styling {
            screen_position: (5.0, 5.0),
            scale: Scale::uniform(32.0f32 * window.hidpi_factor()),
            color: [1.0, 1.0, 1.0, 1.0],
            ..Default::default()
        };
        fps.queue_text(&styling, &mut glyph_brush);

        scene
            .render(&mut encoder, &cam_controller)
            .expect("Could not render scene");

        glyph_brush
            .draw_queued(&mut encoder, &main_color, &main_depth)
            .unwrap();

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();
    }
}

trait GetSeconds {
    fn as_seconds(&self) -> f32;
}

impl GetSeconds for Duration {
    fn as_seconds(&self) -> f32 {
        self.num_nanoseconds().unwrap_or(0) as f32 / 1_000_000_000.0f32
    }
}

pub trait WindowExt {
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
