use std::ops::{Deref, DerefMut};

use crate::{
    ComponentHandle, ComponentManager, Gpu, Isometry, Matrix, Model, ModelBuilder, Rotation,
    Uniform, Vector, Vertex,
};

const MINIMAL_FOV: f32 = 0.0000001;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Camera {
    position: Isometry<f32>,
    fov: Vector<f32>,
    proj: Matrix,
}

impl Camera {
    pub fn new(position: Isometry<f32>, fov: Vector<f32>) -> Self {
        let proj = Matrix::projection(fov);
        Camera {
            position,
            fov,
            proj,
        }
    }

    pub(crate) fn reset_camera_projection(&mut self) {
        self.proj = Matrix::projection(self.fov());
    }

    /// Returns the bottom left and top right corner of the camera. Computes AABB when the camera
    /// is rotated.
    pub fn rect(&self) -> (Vector<f32>, Vector<f32>) {
        fn rotate_point_around_origin(
            origin: Vector<f32>,
            point: Vector<f32>,
            rot: Rotation<f32>,
        ) -> Vector<f32> {
            let sin = rot.sin_angle();
            let cos = rot.cos_angle();
            return Vector::new(
                origin.x + (point.x - origin.x) * cos - (point.y - origin.y) * sin,
                origin.y + (point.x - origin.x) * sin + (point.y - origin.y) * cos,
            );
        }

        let translation = *self.translation();
        let rotation = *self.rotation();
        let fov = self.fov();

        let top_right = rotate_point_around_origin(translation, translation + fov, rotation);
        let bottom_left = rotate_point_around_origin(translation, translation - fov, rotation);
        let top_left = rotate_point_around_origin(
            translation,
            translation + Vector::new(-fov.x, fov.y),
            rotation,
        );
        let bottom_right = rotate_point_around_origin(
            translation,
            translation + Vector::new(fov.x, -fov.y),
            rotation,
        );

        let mut xs = [top_right.x, bottom_left.x, top_left.x, bottom_right.x];
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut ys = [top_right.y, bottom_left.y, top_left.y, bottom_right.y];
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

        return (
            Vector::new(*xs.first().unwrap(), *ys.first().unwrap()),
            Vector::new(*xs.last().unwrap(), *ys.last().unwrap()),
        );
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

    pub fn rotation(&self) -> &Rotation<f32> {
        &self.position.rotation
    }

    pub fn fov(&self) -> Vector<f32> {
        self.fov
    }

    // Setters

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
            model: Model::new(gpu, ModelBuilder::cuboid(fov)),
            uniform: Uniform::new_vertex(gpu, view * proj),
        }
    }

    pub fn write_buffer(&self, gpu: &Gpu, buffer: &mut CameraBuffer) {
        let fov = self.fov();
        let view = self.view();
        let proj = self.proj();
        let vertices = [
            Vertex::new(Vector::new(-fov.x, fov.y), Vector::new(0.0, 0.0)),
            Vertex::new(Vector::new(-fov.x, -fov.y), Vector::new(0.0, 1.0)),
            Vertex::new(Vector::new(fov.x, -fov.y), Vector::new(1.0, 1.0)),
            Vertex::new(Vector::new(fov.x, fov.y), Vector::new(1.0, 0.0)),
        ];
        buffer.model.write_vertices(gpu, &vertices);
        buffer.uniform.write(gpu, view * proj);
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct WorldCamera {
    camera: Camera,
    target: Option<ComponentHandle>,
    horizontal_scale: f32,
    vertical_fov: f32,
}

impl WorldCamera {
    pub fn new(position: Isometry<f32>, vertical_fov: f32, horizontal_scale: f32) -> Self {
        let fov = Vector::new(vertical_fov * horizontal_scale, vertical_fov);
        Self {
            camera: Camera::new(position, fov),
            target: None,
            vertical_fov,
            horizontal_scale,
        }
    }

    pub fn apply_target(&mut self, man: &ComponentManager) {
        if let Some(target) = self.target() {
            if let Some(component) = man.component_dynamic(target) {
                let translation = component.base().translation();
                self.set_translation(translation);
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

    pub fn set_vertical_fov(&mut self, mut new_fov: f32) {
        if new_fov < MINIMAL_FOV {
            new_fov = MINIMAL_FOV;
        }
        self.vertical_fov = new_fov;
        self.compute_fov();
    }

    pub fn set_horizontal_fov(&mut self, mut new_fov: f32) {
        if new_fov < MINIMAL_FOV {
            new_fov = MINIMAL_FOV;
        }
        self.vertical_fov = new_fov / self.horizontal_scale;
        self.compute_fov();
    }

    pub(crate) fn compute_fov(&mut self) {
        let fov = Vector::new(self.vertical_fov * self.horizontal_scale, self.vertical_fov);
        self.set_fov(fov);
    }

    pub(crate) fn resize(&mut self, horizontal_scale: f32) {
        self.horizontal_scale = horizontal_scale;
        self.compute_fov();
    }

    pub fn vertical_fov(&self) -> f32 {
        self.vertical_fov
    }

    pub fn horizontal_fov_scale(&self) -> f32 {
        self.horizontal_scale
    }
}

impl Deref for WorldCamera {
    type Target = Camera;

    fn deref(&self) -> &Self::Target {
        &self.camera
    }
}

impl DerefMut for WorldCamera {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.camera
    }
}

pub struct CameraBuffer {
    pub model: Model,
    pub uniform: Uniform<Matrix>,
}

impl CameraBuffer {
    pub fn uniform(&self) -> &Uniform<Matrix> {
        &self.uniform
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}

pub struct BufferedCamera {
    camera: Camera,
    buffer: CameraBuffer,
}

impl Deref for BufferedCamera {
    type Target = CameraBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl BufferedCamera {
    pub fn new(gpu: &Gpu, camera: Camera) -> BufferedCamera {
        BufferedCamera {
            buffer: camera.create_buffer(gpu),
            camera,
        }
    }

    pub fn write(&mut self, gpu: &Gpu, camera: Camera) {
        self.camera = camera;
        self.camera.write_buffer(gpu, &mut self.buffer)
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn buffer(&self) -> &CameraBuffer {
        &self.buffer
    }
}
