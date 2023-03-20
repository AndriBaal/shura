use std::ops::Range;

use crate::{math::matrix::Matrix, Gpu};
use wgpu::util::DeviceExt;

/// Buffer holding multiple [Positions](crate::Isometry) in form of [Matrices](crate::Matrix).
pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
}

impl InstanceBuffer {
    pub fn new(gpu: &Gpu, data: &[Matrix]) -> Self {
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(data),
            });

        return Self { buffer };
    }

    pub fn write(&self, gpu: &Gpu, data: &[Matrix]) {
        assert_eq!(data.len() as u32, self.amount_of_instances());
        gpu.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..)
    }

    pub fn size(&self) -> u64 {
        self.buffer.size()
    }

    pub fn instances(&self) -> InstanceIndices {
        InstanceIndices {
            range: 0..self.amount_of_instances(),
        }
    }

    pub fn amount_of_instances(&self) -> u32 {
        const MATRIX_SIZE: u64 = std::mem::size_of::<Matrix>() as u64;
        let buffer_size = self.size();
        return (buffer_size / MATRIX_SIZE) as u32;
    }
}

#[derive(Debug, Copy, Clone)]
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
        InstanceIndices {
            range: self
        }
    }
}

#[derive(Debug, Clone)]
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
