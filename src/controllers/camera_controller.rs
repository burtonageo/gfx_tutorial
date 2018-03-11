use ::WindowExt;
use ang::{Angle, Degrees};
use graphics::camera::{Camera, CameraMatrices};
use na::{self, Isometry3, Point3, Perspective3, Vector3};
use num::Zero;
use std::ops::Neg;

#[derive(Clone, Debug, PartialEq)]
pub struct CameraController {
    pub position: Point3<f32>,
    horizontal_angle: Angle<f32>,
    vertical_angle: Angle<f32>,
    fov: Angle<f32>,
    perspective: Perspective3<f32>,
    mouse_speed: f32,
}

impl CameraController {
    pub fn new<W: WindowExt>(
        window: &W,
        mouse_speed: f32,
    ) -> Self {
        let fov = Angle::eighth();
        CameraController {
            position: Point3::new(0.0, 0.0, 10.0),
            horizontal_angle: Angle::zero(),
            vertical_angle: Angle::zero(),
            fov,
            perspective: Perspective3::new(window.aspect(), fov.in_radians(), 0.1, 100.0),
            mouse_speed,
        }
    }

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

    pub fn rotate_view_angles_by(&mut self, delta_h: Angle<f32>, delta_v: Angle<f32>) {
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
}

impl Camera for CameraController {
    fn matrices(&self) -> CameraMatrices {
        let view = {
            let direction = self.direction();
            let right = self.right();
            let up = self.up();

            Isometry3::look_at_lh(
                &self.position,
                &Point3::from_coordinates(self.position.coords + direction),
                &up,
            )
        };

        CameraMatrices {
            view: view.to_homogeneous(),
            projection: self.perspective.to_homogeneous(),
        }
    }
}
