use nalgebra::Vector4;

use crate::Vector;
use std::mem;
use std::ops::*;

/// Single vertex of a model. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex {
    pub pos: Vector<f32>,
    pub tex_coords: Vector<f32>,
}

impl Vertex {
    pub fn new(pos: Vector<f32>, tex_coords: Vector<f32>) -> Self {
        Vertex { pos, tex_coords }
    }

    pub const fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }

    pub const fn instance_desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    pub fn normalize(&self) -> Vertex {
        Vertex::new(self.pos.normalize(), self.tex_coords.normalize())
    }
}

impl Add for Vertex {
    type Output = Vertex;
    fn add(self, v: Vertex) -> Vertex {
        return Vertex {
            pos: self.pos + v.pos,
            tex_coords: self.tex_coords + v.tex_coords,
        };
    }
}

impl Sub for Vertex {
    type Output = Vertex;
    fn sub(self, v: Vertex) -> Vertex {
        return Vertex {
            pos: self.pos - v.pos,
            tex_coords: self.tex_coords - v.tex_coords,
        };
    }
}

impl Div for Vertex {
    type Output = Vertex;
    fn div(self, v: Vertex) -> Vertex {
        return Vertex {
            pos: Vector::new(self.pos.x / v.pos.x, self.pos.y / v.pos.y),
            tex_coords: Vector::new(
                self.tex_coords.x / v.tex_coords.x,
                self.tex_coords.y / v.tex_coords.y,
            ),
        };
    }
}

impl Mul for Vertex {
    type Output = Vertex;
    fn mul(self, v: Vertex) -> Vertex {
        return Vertex {
            pos: Vector::new(self.pos.x * v.pos.x, self.pos.y * v.pos.y),
            tex_coords: Vector::new(
                self.tex_coords.x * v.tex_coords.x,
                self.tex_coords.y * v.tex_coords.y,
            ),
        };
    }
}

impl Rem for Vertex {
    type Output = Vertex;
    fn rem(self, v: Vertex) -> Vertex {
        return Vertex {
            pos: Vector::new(self.pos.x % v.pos.x, self.pos.y % v.pos.y),
            tex_coords: Vector::new(
                self.tex_coords.x % v.tex_coords.x,
                self.tex_coords.y % v.tex_coords.y,
            ),
        };
    }
}

impl Mul<f32> for Vertex {
    type Output = Vertex;
    fn mul(self, v: f32) -> Vertex {
        return Vertex {
            pos: self.pos * v,
            tex_coords: self.tex_coords * v,
        };
    }
}

impl Mul<Vector4<f32>> for Vertex {
    type Output = Vertex;
    fn mul(self, v: Vector4<f32>) -> Vertex {
        let pos = Vector::new(
            self.pos[0] * v[0] + self.pos[1] * v[1],
            self.pos[0] * v[2] + self.pos[1] * v[3],
        );
        let tex_coords = Vector::new(
            self.tex_coords[0] * v[0] + self.tex_coords[1] * v[1],
            self.tex_coords[0] * v[2] + self.tex_coords[1] * v[3],
        );
        Self { pos, tex_coords }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
/// Represents the order in which (Vertices)[Vertex] are draw in a triangle.
pub struct Index {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Index {
    pub fn new(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }
}
