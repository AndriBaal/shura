use std::ops::Deref;

use crate::{
    ComponentHandle, ComponentManager, Gpu, Isometry, Matrix, Model, ModelBuilder, Rotation,
    Uniform, Vector,
};

#[cfg(feature = "physics")]
use crate::physics::World;

const MINIMAL_FOV: f32 = 5.4E-079;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// 2D Camera for rendering
pub struct Camera {
    position: Isometry<f32>,
    fov: Vector<f32>,
    proj: Matrix,
}

impl Camera {
    pub fn new(position: Isometry<f32>, fov: Vector<f32>) -> Self {
        let proj = Matrix::frustum(fov);
        Camera {
            position,
            fov,
            proj,
        }
    }

    pub(crate) fn reset_camera_projection(&mut self) {
        self.proj = Matrix::frustum(self.fov());
    }

    pub const fn position(&self) -> &Isometry<f32> {
        &self.position
    }

    pub const fn translation(&self) -> &Vector<f32> {
        &self.position.translation.vector
    }

    pub fn view(&self) -> Matrix {
        Matrix::view(self.position)
    }

    pub fn proj(&self) -> Matrix {
        self.proj
    }

    pub fn view_proj(&self) -> Matrix {
        self.view() * self.proj()
    }

    pub fn rotation(&self) -> &Rotation<f32> {
        &self.position.rotation
    }

    pub fn fov(&self) -> Vector<f32> {
        self.fov
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.position.rotation = rotation;
    }

    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.position = position;
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.position.translation.vector = translation;
    }

    pub fn set_fov(&mut self, mut new_fov: Vector<f32>) {
        if new_fov.x < MINIMAL_FOV {
            new_fov.x = MINIMAL_FOV;
        }
        if new_fov.y < MINIMAL_FOV {
            new_fov.y = MINIMAL_FOV;
        }
        self.fov = new_fov;
        self.reset_camera_projection();
    }

    pub fn create_buffer(&self, gpu: &Gpu) -> CameraBuffer {
        let fov = self.fov();
        let view = self.view();
        let proj = self.proj();
        CameraBuffer {
            model: Model::new(
                gpu,
                ModelBuilder::cuboid(fov).vertex_position(self.position),
            ),
            uniform: Uniform::new_vertex(gpu, view * proj),
        }
    }

    pub fn write_buffer(&mut self, gpu: &Gpu, buffer: &mut CameraBuffer) {
        let fov = self.fov();
        let view = self.view();
        let proj = self.proj();
        buffer.model.write_vertices(
            gpu,
            &ModelBuilder::cuboid(fov)
                .vertex_position(self.position)
                .apply_modifiers()
                .vertices,
        );
        buffer.uniform.write(gpu, view * proj);
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// Limits a [Camera] to always maintain the aspect ratio of the window. This behaviour can be controlled
/// through the [WorldCameraScale]. This camera is also in charge of deciding which [Groups](crate::Group)
/// is active based on if the [Groups](crate::Group) intersects with the camera.
pub struct WorldCamera {
    pub(crate) camera: Camera,
    target: Option<ComponentHandle>,
    scale: WorldCameraScale,
    window_size: Vector<f32>,
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

    pub fn fov(&self, window_size: Vector<f32>) -> Vector<f32> {
        match self {
            WorldCameraScale::Max(mut max) => {
                if max < MINIMAL_FOV {
                    max = MINIMAL_FOV;
                }

                return if window_size.x > window_size.y {
                    Vector::new(max, window_size.y / window_size.x * max)
                } else {
                    Vector::new(window_size.x / window_size.y * max, max)
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
                    Vector::new(scale * min, min)
                } else {
                    Vector::new(min, scale * min)
                };
            }
            WorldCameraScale::Horizontal(mut horizontal) => {
                if horizontal < MINIMAL_FOV {
                    horizontal = MINIMAL_FOV;
                }

                let yx = window_size.y / window_size.x * horizontal;
                Vector::new(horizontal, yx)
            }
            WorldCameraScale::Vertical(mut vertical) => {
                if vertical < MINIMAL_FOV {
                    vertical = MINIMAL_FOV;
                }

                let xy = window_size.x / window_size.y * vertical;
                Vector::new(xy, vertical)
            }
        }
    }
}

impl WorldCamera {
    pub fn new(position: Isometry<f32>, scale: WorldCameraScale, window_size: Vector<u32>) -> Self {
        let window_size = window_size.cast();
        let fov = scale.fov(window_size);
        Self {
            camera: Camera::new(position, fov),
            target: None,
            window_size,
            scale,
        }
    }

    pub fn apply_target(
        &mut self,
        #[cfg(feature = "physics")] world: &World,
        man: &ComponentManager,
    ) {
        if let Some(target) = self.target() {
            if let Some(component) = man.get_boxed(target) {
                // TODO: Maybe change this to not read from the matrix
                let matrix = component.base().matrix(
                    #[cfg(feature = "physics")]
                    world,
                );
                let translation = Vector::new(matrix[12], matrix[13]);
                self.camera.set_translation(translation);
            } else {
                self.set_target(None);
            }
        }
    }

    pub fn target(&self) -> Option<ComponentHandle> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<ComponentHandle>) {
        self.target = target;
    }

    pub(crate) fn compute_fov(&mut self) {
        let fov = self.scale.fov(self.window_size);
        self.camera.set_fov(fov);
    }

    pub(crate) fn resize(&mut self, window_size: Vector<u32>) {
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

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.camera.position.rotation = rotation;
    }

    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.camera.position = position;
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.camera.position.translation.vector = translation;
    }
}

impl Deref for WorldCamera {
    type Target = Camera;

    fn deref(&self) -> &Self::Target {
        &self.camera
    }
}

/// Holds the [Uniform] with the matrix of a [Camera] and the [Model] of the fov.
pub struct CameraBuffer {
    model: Model,
    uniform: Uniform<Matrix>,
}

impl CameraBuffer {
    pub fn uniform(&self) -> &Uniform<Matrix> {
        &self.uniform
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}
