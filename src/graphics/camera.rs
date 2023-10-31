use nalgebra::Orthographic3;
use std::ops::*;

use crate::AABB;
use crate::{
    ComponentHandle, ComponentManager, Gpu, Isometry2, Isometry3, Matrix4, Rotation2, Uniform,
    Vector2, Vector3, World,
};

const MINIMAL_FOV: f32 = 0.0001;

pub trait Camera {
    fn uniform(&self) -> &Uniform<Matrix4<f32>>;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug)]
/// 2D Camera for rendering
pub struct Camera2D {
    position: Isometry2<f32>,
    fov: Vector2<f32>,
    proj: Orthographic3<f32>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    uniform: Option<Uniform<Matrix4<f32>>>,
}

impl Clone for Camera2D {
    fn clone(&self) -> Self {
        Self {
            position: self.position.clone(),
            fov: self.fov.clone(),
            proj: self.proj.clone(),
            uniform: None,
        }
    }
}

impl Camera2D {
    pub fn new_unbuffered(position: Isometry2<f32>, fov: Vector2<f32>) -> Self {
        let proj = Orthographic3::new(-fov.x, fov.x, -fov.y, fov.y, -1.0, 1.0);
        Camera2D {
            position,
            fov,
            proj,
            uniform: None,
        }
    }

    pub fn new(gpu: &Gpu, position: Isometry2<f32>, fov: Vector2<f32>) -> Self {
        let mut camera = Self::new_unbuffered(position, fov);
        camera.buffer(gpu);
        return camera;
    }

    pub(crate) fn reset_camera_projection(&mut self) {
        let fov = self.fov();
        self.proj = Orthographic3::new(-fov.x, fov.x, -fov.y, fov.y, -1.0, 1.0);
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
        self.reset_camera_projection();
    }

    pub fn buffer(&mut self, gpu: &Gpu) {
        let view_proj = self.view_proj();
        if let Some(uniform) = &mut self.uniform {
            uniform.write(gpu, view_proj);
        } else {
            self.uniform = Some(Uniform::new_vertex(gpu, view_proj));
        }
    }
}

impl Camera for Camera2D {
    fn uniform(&self) -> &Uniform<Matrix4<f32>> {
        self.uniform
            .as_ref()
            .expect("Camera buffer not initialized!")
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// Limits a [Camera] to always maintain the aspect ratio of the window. This behaviour can be controlled
/// through the [WorldCameraScale]. This camera is also in charge of deciding which [Groups](crate::Group)
/// is active based on if the [Groups](crate::Group) intersects with the camera.
pub struct WorldCamera2D {
    pub(crate) camera: Camera2D,
    scale: WorldCameraScale,
    window_size: Vector2<f32>,
}


impl WorldCamera2D {
    pub fn new(
        position: Isometry2<f32>,
        scale: WorldCameraScale,
        window_size: Vector2<u32>,
    ) -> Self {
        let window_size = window_size.cast();
        let fov = scale.fov(window_size);
        Self {
            camera: Camera2D::new_unbuffered(position, fov),
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

    pub fn fov_scale(&self) -> WorldCameraScale {
        self.scale
    }

    pub fn set_scaling(&mut self, scale: WorldCameraScale) {
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

    pub fn buffer(&mut self, gpu: &Gpu) {
        self.camera.buffer(gpu)
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
pub enum WorldCameraScale {
    Max(f32),
    Horizontal(f32),
    Vertical(f32),
    Min(f32),
}

impl WorldCameraScale {
    pub fn value(&self) -> f32 {
        match self {
            WorldCameraScale::Max(max) => *max,
            WorldCameraScale::Min(min) => *min,
            WorldCameraScale::Vertical(vertical) => *vertical,
            WorldCameraScale::Horizontal(horizontal) => *horizontal,
        }
    }

    pub fn fov(&self, window_size: Vector2<f32>) -> Vector2<f32> {
        match self {
            WorldCameraScale::Max(mut max) => {
                if max < MINIMAL_FOV {
                    max = MINIMAL_FOV;
                }

                return if window_size.x > window_size.y {
                    Vector2::new(max, window_size.y / window_size.x * max)
                } else {
                    Vector2::new(window_size.x / window_size.y * max, max)
                };
            }
            WorldCameraScale::Min(mut min) => {
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
            WorldCameraScale::Horizontal(mut horizontal) => {
                if horizontal < MINIMAL_FOV {
                    horizontal = MINIMAL_FOV;
                }

                let yx = window_size.y / window_size.x * horizontal;
                Vector2::new(horizontal, yx)
            }
            WorldCameraScale::Vertical(mut vertical) => {
                if vertical < MINIMAL_FOV {
                    vertical = MINIMAL_FOV;
                }

                let xy = window_size.x / window_size.y * vertical;
                Vector2::new(xy, vertical)
            }
        }
    }
}
