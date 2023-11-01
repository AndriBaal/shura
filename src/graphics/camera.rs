use nalgebra::{Orthographic3, Perspective3};
use std::marker::PhantomData;
use std::ops::*;

use crate::AABB;
use crate::{Gpu, Isometry2, Isometry3, Matrix4, Point3, Rotation2, Uniform, Vector2, Vector3};

const MINIMAL_FOV: f32 = 0.0001;

pub type CameraBuffer2D = CameraBuffer<Camera2D>;
pub struct CameraBuffer<C: Camera> {
    uniform: Uniform<Matrix4<f32>>,
    marker: PhantomData<C>,
}

impl<C: Camera> CameraBuffer<C> {
    pub fn new_camera(gpu: &Gpu, camera: C) -> (Self, C) {
        return (Self::new(gpu, &camera), camera);
    }

    pub fn new(gpu: &Gpu, camera: &C) -> Self {
        Self {
            uniform: Uniform::new_vertex(gpu, camera.matrix()),
            marker: PhantomData,
        }
    }

    pub fn write(&mut self, gpu: &Gpu, camera: &C) {
        self.uniform.write(gpu, camera.matrix());
    }

    pub fn uniform(&self) -> &Uniform<Matrix4<f32>> {
        &self.uniform
    }
}

pub trait Camera {
    fn matrix(&self) -> Matrix4<f32>;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct Camera2D {
    position: Isometry2<f32>,
    fov: Vector2<f32>,
    proj: Orthographic3<f32>,
}

impl Clone for Camera2D {
    fn clone(&self) -> Self {
        Self {
            position: self.position.clone(),
            fov: self.fov.clone(),
            proj: self.proj.clone(),
        }
    }
}

impl Camera2D {
    pub fn new(position: Isometry2<f32>, fov: Vector2<f32>) -> Self {
        let proj = Orthographic3::new(-fov.x, fov.x, -fov.y, fov.y, -1.0, 1.0);
        Camera2D {
            position,
            fov,
            proj,
        }
    }

    pub fn aabb(&self) -> AABB {
        return AABB::from_position(self.fov(), self.position);
    }

    pub const fn position(&self) -> &Isometry2<f32> {
        &self.position
    }

    pub const fn translation(&self) -> &Vector2<f32> {
        &self.position.translation.vector
    }

    pub fn view(&self) -> Isometry3<f32> {
        return Isometry3::new(
            Vector3::new(-self.translation().x, -self.translation().y, 0.0),
            Vector3::new(0.0, 0.0, self.rotation().angle()),
        );
    }

    pub fn proj(&self) -> &Orthographic3<f32> {
        &self.proj
    }

    pub fn view_proj(&self) -> Matrix4<f32> {
        self.proj().as_matrix() * self.view().to_matrix()
    }

    pub fn rotation(&self) -> &Rotation2<f32> {
        &self.position.rotation
    }

    pub fn fov(&self) -> Vector2<f32> {
        self.fov
    }

    pub fn set_rotation(&mut self, rotation: Rotation2<f32>) {
        self.position.rotation = rotation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.position = position;
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.position.translation.vector = translation;
    }

    pub fn set_fov(&mut self, mut new_fov: Vector2<f32>) {
        if new_fov.x < MINIMAL_FOV {
            new_fov.x = MINIMAL_FOV;
        }
        if new_fov.y < MINIMAL_FOV {
            new_fov.y = MINIMAL_FOV;
        }
        self.fov = new_fov;
        self.proj = Orthographic3::new(-new_fov.x, new_fov.x, -new_fov.y, new_fov.y, -1.0, 1.0);
    }
}

impl Camera for Camera2D {
    fn matrix(&self) -> Matrix4<f32> {
        self.view_proj()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// Limits a [Camera] to always maintain the aspect ratio of the window. This behaviour can be controlled
/// through the [WorldCameraScaling]. This camera is also in charge of deciding which [Groups](crate::Group)
/// is active based on if the [Groups](crate::Group) intersects with the camera.
pub struct WorldCamera2D {
    pub(crate) camera: Camera2D,
    scale: WorldCameraScaling,
    window_size: Vector2<f32>,
}

impl WorldCamera2D {
    pub fn new(
        position: Isometry2<f32>,
        scale: WorldCameraScaling,
        window_size: Vector2<u32>,
    ) -> Self {
        let window_size = window_size.cast();
        let fov = scale.fov(window_size);
        Self {
            camera: Camera2D::new(position, fov),
            window_size,
            scale,
        }
    }

    pub(crate) fn compute_fov(&mut self) {
        let fov = self.scale.fov(self.window_size);
        self.camera.set_fov(fov);
    }

    pub(crate) fn resize(&mut self, window_size: Vector2<u32>) {
        self.window_size = window_size.cast();
        self.compute_fov();
    }

    pub fn fov_scale(&self) -> WorldCameraScaling {
        self.scale
    }

    pub fn set_scaling(&mut self, scale: WorldCameraScaling) {
        self.scale = scale;
        self.compute_fov();
    }

    pub fn set_rotation(&mut self, rotation: Rotation2<f32>) {
        self.camera.set_rotation(rotation);
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.camera.set_position(position);
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.camera.set_translation(translation);
    }

    pub fn camera(&self) -> &Camera2D {
        &self.camera
    }
}

impl Deref for WorldCamera2D {
    type Target = Camera2D;

    fn deref(&self) -> &Self::Target {
        &self.camera
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
pub enum WorldCameraScaling {
    Max(f32),
    Horizontal(f32),
    Vertical(f32),
    Min(f32),
}

impl WorldCameraScaling {
    pub fn value(&self) -> f32 {
        match self {
            WorldCameraScaling::Max(max) => *max,
            WorldCameraScaling::Min(min) => *min,
            WorldCameraScaling::Vertical(vertical) => *vertical,
            WorldCameraScaling::Horizontal(horizontal) => *horizontal,
        }
    }

    pub fn fov(&self, window_size: Vector2<f32>) -> Vector2<f32> {
        match self {
            WorldCameraScaling::Max(mut max) => {
                if max < MINIMAL_FOV {
                    max = MINIMAL_FOV;
                }

                return if window_size.x > window_size.y {
                    Vector2::new(max, window_size.y / window_size.x * max)
                } else {
                    Vector2::new(window_size.x / window_size.y * max, max)
                };
            }
            WorldCameraScaling::Min(mut min) => {
                if min < MINIMAL_FOV {
                    min = MINIMAL_FOV;
                }

                let yx = window_size.y / window_size.x;
                let xy = window_size.x / window_size.y;
                let scale = yx.max(xy);
                return if window_size.x > window_size.y {
                    Vector2::new(scale * min, min)
                } else {
                    Vector2::new(min, scale * min)
                };
            }
            WorldCameraScaling::Horizontal(mut horizontal) => {
                if horizontal < MINIMAL_FOV {
                    horizontal = MINIMAL_FOV;
                }

                let yx = window_size.y / window_size.x * horizontal;
                Vector2::new(horizontal, yx)
            }
            WorldCameraScaling::Vertical(mut vertical) => {
                if vertical < MINIMAL_FOV {
                    vertical = MINIMAL_FOV;
                }

                let xy = window_size.x / window_size.y * vertical;
                Vector2::new(xy, vertical)
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
pub struct PerspectiveCamera3D {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub aspect: f32,
}

impl Default for PerspectiveCamera3D {
    fn default() -> Self {
        Self {
            eye: Point3::new(0.0, 1.0, 2.0),
            target: Default::default(),
            up: Vector3::y(),
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
            aspect: 1.0,
        }
    }
}

impl PerspectiveCamera3D {
    pub fn new(window_size: Vector2<u32>) -> Self {
        Self {
            aspect: window_size.x as f32 / window_size.y as f32,
            ..Default::default()
        }
    }

    pub fn resize(&mut self, window_size: Vector2<u32>) {
        self.aspect = window_size.x as f32 / window_size.y as f32;
    }

    pub fn view(&self) -> Isometry3<f32> {
        Isometry3::look_at_rh(&self.eye, &self.target, &self.up)
    }

    pub fn proj(&self) -> Perspective3<f32> {
        Perspective3::new(self.aspect, self.fovy, self.znear, self.zfar)
    }

    pub fn view_proj(&self) -> Matrix4<f32> {
        self.proj().to_homogeneous() * self.view().to_homogeneous()
    }
}

impl Camera for PerspectiveCamera3D {
    fn matrix(&self) -> Matrix4<f32> {
        self.view_proj()
    }
}

pub struct FirstPersonCamera3D {}

// pub struct OrthographicCamera3D {

// }
