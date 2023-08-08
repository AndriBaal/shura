use nalgebra::Vector4;
use std::mem;
use std::ops::*;

use crate::{
    ComponentHandle, ComponentManager, Gpu, Isometry, Model, ModelBuilder, Rotation, Uniform,
    Vector,
};

#[cfg(feature = "physics")]
use crate::physics::World;

const MINIMAL_FOV: f32 = 0.0001;

// Trait to avoid
pub trait CursorCompute {
    fn fov(&self) -> Vector<f32>;
    fn translation(&self) -> Vector<f32>;
}

impl CursorCompute for WorldCamera {
    fn fov(&self) -> Vector<f32> {
        self.fov()
    }

    fn translation(&self) -> Vector<f32> {
        *self.translation()
    }
}

impl CursorCompute for Camera {
    fn fov(&self) -> Vector<f32> {
        self.fov()
    }

    fn translation(&self) -> Vector<f32> {
        *self.translation()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
/// 2D Camera for rendering
pub struct Camera {
    position: Isometry<f32>,
    fov: Vector<f32>,
    proj: CameraMatrix,
}

impl Camera {
    pub fn new(position: Isometry<f32>, fov: Vector<f32>) -> Self {
        let proj = CameraMatrix::frustum(fov);
        Camera {
            position,
            fov,
            proj,
        }
    }

    pub(crate) fn reset_camera_projection(&mut self) {
        self.proj = CameraMatrix::frustum(self.fov());
    }

    pub const fn position(&self) -> &Isometry<f32> {
        &self.position
    }

    pub const fn translation(&self) -> &Vector<f32> {
        &self.position.translation.vector
    }

    pub fn view(&self) -> CameraMatrix {
        CameraMatrix::view(self.position)
    }

    pub fn proj(&self) -> CameraMatrix {
        self.proj
    }

    pub fn view_proj(&self) -> CameraMatrix {
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
            if let Some(instance) = man.instance_data(
                target,
                #[cfg(feature = "physics")]
                world,
            ) {
                self.camera.set_translation(instance.pos());
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

    pub fn position(&self) -> &Isometry<f32> {
        self.camera.position()
    }

    pub fn translation(&self) -> &Vector<f32> {
        self.camera.translation()
    }

    pub fn view(&self) -> CameraMatrix {
        self.camera.view()
    }

    pub fn proj(&self) -> CameraMatrix {
        self.camera.proj()
    }

    pub fn view_proj(&self) -> CameraMatrix {
        self.camera.view_proj()
    }

    pub fn rotation(&self) -> &Rotation<f32> {
        self.camera.rotation()
    }

    pub fn fov(&self) -> Vector<f32> {
        self.camera.fov()
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy)]
/// Defines how the [WorldCamera] gets scaled. This ensures that your game is responsive on various
/// screens.
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

/// Holds the [Uniform] with the matrix of a [Camera] and the [Model] of the fov.
pub struct CameraBuffer {
    model: Model,
    uniform: Uniform<CameraMatrix>,
}

impl CameraBuffer {
    pub fn uniform(&self) -> &Uniform<CameraMatrix> {
        &self.uniform
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CameraMatrix {
    pub x: Vector4<f32>,
    pub y: Vector4<f32>,
    pub z: Vector4<f32>,
    pub w: Vector4<f32>,
}

impl CameraMatrix {
    const NEAR: f32 = 0.1;
    const FAR: f32 = 1.0;
    // Matrix::from_look(Vec3::new(0.0, 0.0,-0.1.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    pub(crate) const NULL_VIEW: CameraMatrix = CameraMatrix::raw(
        Vector4::new(-1.0, 0.0, -0.0, 0.0),
        Vector4::new(0.0, 1.0, -0.0, 0.0),
        Vector4::new(0.0, -0.0, -1.0, 0.0),
        Vector4::new(0.0, 0.0, -Self::NEAR, 1.0),
    );

    pub const fn raw(x: Vector4<f32>, y: Vector4<f32>, z: Vector4<f32>, w: Vector4<f32>) -> Self {
        Self { x, y, z, w }
    }

    /// Frustum
    pub fn frustum(half_extents: Vector<f32>) -> Self {
        let left = -half_extents.x;
        let right = half_extents.x;
        let bottom = -half_extents.y;
        let top = half_extents.y;
        let r_width = 1.0 / (left - right);
        let r_height = 1.0 / (top - bottom);
        let r_depth = 1.0 / (Self::NEAR - Self::FAR);
        let x = 2.0 * (Self::NEAR * r_width);
        let y = 2.0 * (Self::NEAR * r_height);
        let a = (left + right) * r_width;
        let b = (top + bottom) * r_height;
        let c = (Self::FAR + Self::NEAR) * r_depth;
        let d = Self::FAR * Self::NEAR * r_depth;

        Self::raw(
            Vector4::new(x, 0.0, 0.0, 0.0),
            Vector4::new(0.0, y, 0.0, 0.0),
            Vector4::new(a, b, c, -1.0),
            Vector4::new(0.0, 0.0, d, 0.0),
        )
    }

    /// View
    pub fn view(mut position: Isometry<f32>) -> Self {
        let mut result = Self::NULL_VIEW;
        let s = position.rotation.sin_angle();
        let c = position.rotation.cos_angle();
        let temp = position.translation;
        position.translation.x = temp.x * c - (temp.y) * s;
        position.translation.y = temp.x * s + (temp.y) * c;

        result[12] = position.translation.x;
        result[13] = -position.translation.y;
        result[0] = -c;
        result[1] = s;
        result[4] = s;
        result[5] = c;

        return result;
    }

    pub fn ortho(dim: Vector<f32>) -> Self {
        Self::raw(
            Vector4::new(2.0 / dim.x, 0.0, 0.0, 0.0),
            Vector4::new(0.0, -2.0 / dim.y, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(-1.0, 1.0, 0.0, 1.),
        )
    }
}

impl AsRef<[f32; 16]> for CameraMatrix {
    fn as_ref(&self) -> &[f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl AsMut<[f32; 16]> for CameraMatrix {
    fn as_mut(&mut self) -> &mut [f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl Into<[f32; 16]> for CameraMatrix {
    fn into(self) -> [f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl Index<usize> for CameraMatrix {
    type Output = f32;

    fn index<'a>(&'a self, i: usize) -> &'a f32 {
        let v: &[f32; 16] = self.as_ref();
        &v[i]
    }
}

impl IndexMut<usize> for CameraMatrix {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut f32 {
        let v: &mut [f32; 16] = self.as_mut();
        &mut v[i]
    }
}

impl Mul for CameraMatrix {
    type Output = CameraMatrix;
    fn mul(self, m: CameraMatrix) -> Self {
        CameraMatrix::raw(
            m.x * self[0] + m.y * self[1] + m.z * self[2] + m.w * self[3],
            m.x * self[4] + m.y * self[5] + m.z * self[6] + m.w * self[7],
            m.x * self[8] + m.y * self[9] + m.z * self[10] + m.w * self[11],
            m.x * self[12] + m.y * self[13] + m.z * self[14] + m.w * self[15],
        )
    }
}
