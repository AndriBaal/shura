use crate::Vector;
use std::mem;

/// Single vertex of a model. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex<T: bytemuck::Pod + bytemuck::Zeroable + Default> {
    pub pos: Vector<f32>,
    pub tex_coords: Vector<f32>,
    pub additional: T,
}

// Use manual implementation instead of derive because of T
unsafe impl<T: bytemuck::Pod + Default> bytemuck::Pod for Vertex<T> {}
unsafe impl<T: bytemuck::Zeroable + bytemuck::Pod + Default> bytemuck::Zeroable for Vertex<T> {}

impl<T: bytemuck::Pod + bytemuck::Zeroable + Default> Vertex<T> {
    pub const fn new(pos: Vector<f32>, tex_coords: Vector<f32>, additional: T) -> Self {
        Vertex {
            pos,
            tex_coords,
            additional,
        }
    }
}

impl Vertex<()> {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
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
    ];
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBUTES,
    };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Represents the order in which [Vertices](Vertex) are drawn in a triangle.
pub struct Index {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Index {
    pub const fn new(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }
}
