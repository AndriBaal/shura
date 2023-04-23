use crate::{Isometry, Rotation, Vector};
use nalgebra::Vector4;
use std::mem;
use std::ops::*;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Matrix optimized for 2D rendering in shura.
pub struct Matrix {
    pub x: Vector4<f32>,
    pub y: Vector4<f32>,
    pub z: Vector4<f32>,
    pub w: Vector4<f32>,
}

impl Matrix {
    // Matrix::from_look(Vec3::new(0.0, 0.0,-3.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    pub(crate) const NULL_VIEW: Matrix = Matrix::raw(
        Vector4::new(-1.0, 0.0, -0.0, 0.0),
        Vector4::new(0.0, 1.0, -0.0, 0.0),
        Vector4::new(0.0, -0.0, -1.0, 0.0),
        Vector4::new(0.0, 0.0, -3.0, 1.0),
    );

    pub const fn raw(x: Vector4<f32>, y: Vector4<f32>, z: Vector4<f32>, w: Vector4<f32>) -> Matrix {
        Self { x, y, z, w }
    }

    pub fn new(pos: Isometry<f32>, scale: Vector<f32>) -> Self {
        let mut matrix = Matrix::default();
        matrix.translate(pos.translation.vector);
        matrix.rotate(scale, pos.rotation);
        return matrix;
    }

    pub fn translate(&mut self, pos: Vector<f32>) {
        self[12] = pos.x;
        self[13] = pos.y;
    }

    pub fn rotate(&mut self, scale: Vector<f32>, rotation: Rotation<f32>) {
        let s = rotation.sin_angle();
        let c = rotation.cos_angle();
        self[0] = c * scale.x;
        self[1] = s * scale.x;
        self[4] = -s * scale.y;
        self[5] = c * scale.y;
    }

    /// Frustum
    pub fn projection(half_extents: Vector<f32>) -> Matrix {
        const NEAR: f32 = 3.0;
        const FAR: f32 = 7.0;
        let left = -half_extents.x;
        let right = half_extents.x;
        let bottom = -half_extents.y;
        let top = half_extents.y;
        let r_width = 1.0 / (left - right);
        let r_height = 1.0 / (top - bottom);
        let r_depth = 1.0 / (NEAR - FAR);
        let x = 2.0 * (NEAR * r_width);
        let y = 2.0 * (NEAR * r_height);
        let a = (left + right) * r_width;
        let b = (top + bottom) * r_height;
        let c = (FAR + NEAR) * r_depth;
        let d = FAR * NEAR * r_depth;

        Self::raw(
            Vector4::new(x, 0.0, 0.0, 0.0),
            Vector4::new(0.0, y, 0.0, 0.0),
            Vector4::new(a, b, c, -1.0),
            Vector4::new(0.0, 0.0, d, 0.0),
        )
    }

    /// View
    pub fn view(mut position: Isometry<f32>) -> Matrix {
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
}

impl Default for Matrix {
    fn default() -> Matrix {
        Self {
            x: Vector4::new(1.0, 0.0, 0.0, 0.0),
            y: Vector4::new(0.0, 1.0, 0.0, 0.0),
            z: Vector4::new(0.0, 0.0, 0.5, 0.0),
            w: Vector4::new(0.0, 0.0, 0.5, 1.0),
        }
    }
}

impl AsRef<[f32; 16]> for Matrix {
    fn as_ref(&self) -> &[f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl AsMut<[f32; 16]> for Matrix {
    fn as_mut(&mut self) -> &mut [f32; 16] {
        unsafe { mem::transmute(self) }
    }
}

impl Index<usize> for Matrix {
    type Output = f32;

    fn index<'a>(&'a self, i: usize) -> &'a f32 {
        let v: &[f32; 16] = self.as_ref();
        &v[i]
    }
}

impl IndexMut<usize> for Matrix {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut f32 {
        let v: &mut [f32; 16] = self.as_mut();
        &mut v[i]
    }
}

impl Mul for Matrix {
    type Output = Matrix;
    fn mul(self, m: Matrix) -> Matrix {
        let a = m.x;
        let b = m.y;
        let c = m.z;
        let d = m.w;

        Matrix::raw(
            a * self[0] + b * self[1] + c * self[2] + d * self[3],
            a * self[4] + b * self[5] + c * self[6] + d * self[7],
            a * self[8] + b * self[9] + c * self[10] + d * self[11],
            a * self[12] + b * self[13] + c * self[14] + d * self[15],
        )
    }
}

impl Into<Matrix> for Isometry<f32> {
    fn into(self) -> Matrix {
        Matrix::new(self, Vector::new(1.0, 1.0))
    }
}
