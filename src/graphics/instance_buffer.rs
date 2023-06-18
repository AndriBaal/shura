use std::{mem, ops::Range};

use crate::{Gpu, Isometry, Matrix, Rotation, Vector};
use wgpu::util::DeviceExt;

/// Single vertex of a model. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstanceData {
    pos: Vector<f32>,
    rot: Matrix<f32>,
}

impl InstanceData {
    pub fn new(pos: Isometry<f32>, scale: Vector<f32>) -> Self {
        Self {
            rot: Matrix::new(scale.x, 0.0, 0.0, scale.y)
                * Matrix::new(
                    pos.rotation.cos_angle(),
                    -pos.rotation.sin_angle(),
                    pos.rotation.sin_angle(),
                    pos.rotation.cos_angle(),
                ),
            pos: pos.translation.vector,
        }
    }

    pub const fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<Vector<f32>>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.pos = translation;
    }

    pub fn set_scale_rotation(&mut self, scale: Vector<f32>, rotation: Rotation<f32>) {
        self.rot = Matrix::new(scale.x, 0.0, 0.0, scale.y)
            * Matrix::new(
                rotation.cos_angle(),
                -rotation.sin_angle(),
                rotation.sin_angle(),
                rotation.cos_angle(),
            )
    }

    pub fn pos(&self) -> Vector<f32> {
        self.pos
    }
}

impl Default for InstanceData {
    fn default() -> Self {
        return Self::new(Isometry::new(Vector::default(), 0.0), Vector::new(1.0, 1.0));
    }
}

/// Buffer holding multiple [Positions](crate::Isometry) in form of [Matrices](crate::Matrix).
pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
    instances: u32,
}

impl InstanceBuffer {
    pub fn new(gpu: &Gpu, data: &[InstanceData]) -> Self {
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(data),
            });

        return Self {
            buffer,
            instances: data.len() as u32,
        };
    }

    pub fn empty(gpu: &Gpu, amount: u32) -> Self {
        let buffer = gpu
            .device
            .create_buffer(&wgpu::BufferDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: std::mem::size_of::<InstanceData>() as u64 * amount as u64,
                mapped_at_creation: false,
            });

        return Self {
            buffer,
            instances: amount
        };
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[InstanceData]) {
        self.instances = data.len() as u32;
        gpu.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.size())
    }

    pub fn size(&self) -> u64 {
        self.buffer.size()
    }

    pub fn instances(&self) -> InstanceIndices {
        InstanceIndices {
            range: 0..self.len(),
        }
    }

    pub fn len(&self) -> u32 {
        self.instances
    }

    pub fn capacity(&self) -> u32 {
        const INSTANCE_SIZE: u64 = std::mem::size_of::<InstanceData>() as u64;
        let buffer_size = self.size();
        return (buffer_size / INSTANCE_SIZE) as u32;
    }
}

#[derive(Debug, Copy, Clone)]
/// Index of a [Position](crate::Isometry) in a [InstanceBuffer] represented by a [Matrix]
pub struct InstanceIndex {
    pub index: u32,
}

impl InstanceIndex {
    pub const fn new(index: u32) -> Self {
        Self { index }
    }
}

impl Into<InstanceIndices> for InstanceIndex {
    fn into(self) -> InstanceIndices {
        InstanceIndices {
            range: self.index..self.index + 1,
        }
    }
}

impl Into<InstanceIndices> for u32 {
    fn into(self) -> InstanceIndices {
        InstanceIndices {
            range: self..self + 1,
        }
    }
}

impl Into<InstanceIndices> for Range<u32> {
    fn into(self) -> InstanceIndices {
        InstanceIndices { range: self }
    }
}

#[derive(Debug, Clone)]
/// Range of [InstanceIndex]
pub struct InstanceIndices {
    pub range: Range<u32>,
}

impl InstanceIndices {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { range: start..end }
    }
}

impl Iterator for InstanceIndices {
    type Item = InstanceIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.range.next() {
            return Some(InstanceIndex::new(index));
        }
        return None;
    }
}
