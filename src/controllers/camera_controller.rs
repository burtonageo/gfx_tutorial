use ::WindowExt;
use ang::{Angle, Degrees};
use graphics::camera::{Camera, CameraMatrices};
use na::{self, Isometry3, Point3, Perspective3, Vector3};
use num::Zero;
use std::ops::Neg;

#[derive(Clone, Debug, PartialEq)]
pub struct CameraController {
    pub input: PlayerInput,
    position: Point3<f32>,
    horizontal_angle: Angle<f32>,
    vertical_angle: Angle<f32>,
    fov: Angle<f32>,
    perspective: Perspective3<f32>,
    mouse_speed: f32,
    move_speed: f32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlayerInput {
    pub moving_forwards: bool,
    pub moving_backwards: bool,
    pub moving_left: bool,
    pub moving_right: bool,
    pub moving_up: bool,
    pub moving_down: bool,
}

impl CameraController {
    pub fn new<W: WindowExt>(
        window: &W,
        mouse_speed: f32,
        move_speed: f32,
    ) -> Self {
        let fov = Angle::eighth();
        CameraController {
            input: Default::default(),
            position: Point3::new(0.0, 0.0, 10.0),
            horizontal_angle: Angle::zero(),
            vertical_angle: Angle::zero(),
            fov,
            perspective: Perspective3::new(window.aspect(), fov.in_radians(), 0.1, 100.0),
            mouse_speed,
            move_speed,
        }
    }

    #[inline]
    pub fn on_resize<W: WindowExt>(&mut self, window: &W) {
        self.perspective.set_aspect(window.aspect());
    }

    #[inline]
    pub fn direction(&self) -> Vector3<f32> {
        Vector3::new(
            self.vertical_angle.cos() * self.horizontal_angle.sin(),
            self.vertical_angle.sin(),
            self.vertical_angle.cos() * self.horizontal_angle.cos(),
        )
    }

    #[inline]
    pub fn right(&self) -> Vector3<f32> {
        Vector3::new(
            (self.horizontal_angle - Angle::quarter()).sin(),
            na::zero(),
            (self.horizontal_angle - Angle::quarter()).cos(),
        )
    }

    #[inline]
    pub fn up(&self) -> Vector3<f32> {
        self.right().cross(&self.direction())
    }

    pub fn on_cursor_moved(&mut self, dt_s: f32, (x, y): (f64, f64), (win_w, win_h): (i32, i32), hidpi: f32) {
        let delta_h = Degrees(self.mouse_speed * dt_s * ((win_w / 2) as f32 - (x as f32 / hidpi)));
        let delta_v = Degrees(self.mouse_speed * dt_s * ((win_h / 2) as f32 - (y as f32 / hidpi)));

        self.horizontal_angle += delta_h;
        self.vertical_angle -= delta_v;

        self.horizontal_angle = self.horizontal_angle.normalized();

        let threshold = Angle::quarter() - Degrees(1.0f32);

        if self.vertical_angle > threshold {
            self.vertical_angle = threshold;
        }

        if self.vertical_angle < threshold.neg() {
            self.vertical_angle = threshold.neg();
        }
    }

    pub fn apply_input(&mut self, dt_s: f32) {
        let direction = self.direction();
        let right = self.right();
        let up =Vector3::new(0.0, 1.0, 0.0);
        if self.input.moving_forwards {
            self.position -= direction * self.move_speed * dt_s;
        }
        if self.input.moving_backwards {
            self.position += direction * self.move_speed * dt_s;
        }
        if self.input.moving_left {
            self.position += right * self.move_speed * dt_s;
        }
        if self.input.moving_right {
            self.position -= right * self.move_speed * dt_s;
        }
        if self.input.moving_up {
            self.position += up * self.move_speed * dt_s;
        }
        if self.input.moving_down {
            self.position -= up * self.move_speed * dt_s;
        }
    }
}

impl Camera for CameraController {
    fn matrices(&self) -> CameraMatrices {
        let view = {
            let direction = self.direction();
            let up = self.up();

            Isometry3::look_at_lh(
                &self.position,
                &Point3::from_coordinates(self.position.coords + direction),
                &up,
            )
        };

        CameraMatrices::new(view.to_homogeneous(), self.perspective.to_homogeneous())
    }
}
