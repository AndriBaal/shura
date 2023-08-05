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
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    pub fn new(pos: Isometry<f32>, scale: Vector<f32>) -> Self {
        Self {
            rot: Matrix::new(
                scale.x * pos.rotation.cos_angle(),
                scale.x * pos.rotation.sin_angle(),
                scale.y * -pos.rotation.sin_angle(),
                scale.y * pos.rotation.cos_angle(),
            ),
            pos: pos.translation.vector,
        }
    }

    // pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
    //     const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
    //         5 => Float32x2,
    //         6 => Float32x4
    //     ];
    //     wgpu::VertexBufferLayout {
    //         array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
    //         step_mode: wgpu::VertexStepMode::Instance,
    //         attributes: &ATTRIBUTES,
    //     }
    // }

    pub fn size() -> wgpu::BufferAddress {
        mem::size_of::<Self>() as wgpu::BufferAddress
    }

    pub fn attributes() -> Vec<wgpu::VertexAttribute> {
        wgpu::vertex_attr_array![
            5 => Float32x2,
            6 => Float32x4
        ]
        .to_vec()
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
    instances: u64,
    instance_size: u64,
}

impl InstanceBuffer {
    pub fn new(gpu: &Gpu, instance_size: u64, data: &[u8]) -> Self {
        assert!(data.len() as u64 % instance_size == 0);
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: data,
            });

        return Self {
            buffer,
            instances: data.len() as u64 / instance_size,
            instance_size,
        };
    }

    pub fn empty(gpu: &Gpu, instance_size: u64, amount: u64) -> Self {
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: instance_size as u64 * amount as u64,
            mapped_at_creation: false,
        });

        return Self {
            buffer,
            instance_size,
            instances: 0,
        };
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[u8]) {
        self.instances = data.len() as u64 / self.instance_size;
        self.write_offset(gpu, 0, data);

        // if data.len() as u64 > self.buffer.size() {
        //     *self = Self::new(gpu, self.instance_size, data)
        // } else {
        //     self.instances = data.len() as u64 / self.instance_size;
        //     self.write_offset(gpu, 0, data);
        // }
    }

    pub fn write_offset(&mut self, gpu: &Gpu, instance_offset: u64, data: &[u8]) {
        assert!(data.len() as u64 % self.instance_size == 0);
        assert!(instance_offset * self.instance_size + data.len() as u64 <= self.buffer.size());
        gpu.queue.write_buffer(&self.buffer, instance_offset * self.instance_size, data);
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.instance_size * self.instances)
    }

    pub fn buffer_capacity(&self) -> u64 {
        self.buffer.size()
    }

    pub fn instances(&self) -> InstanceIndices {
        InstanceIndices {
            range: 0..self.instance_amount() as u32,
        }
    }

    pub fn instance_capacity(&self) -> u64 {
        return self.buffer_capacity() / self.instance_size;
    }

    pub fn instance_amount(&self) -> u64 {
        self.instances
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
