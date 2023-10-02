use std::{mem::size_of, ops::Range};

use crate::{Gpu, Isometry, Matrix, Rotation, Vector};
use wgpu::util::DeviceExt;

/// Single vertex of a model. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstancePosition {
    pos: Vector<f32>,
    rot: Matrix<f32>,
}

impl InstancePosition {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        2 => Float32x2,
        3 => Float32x4
    ];
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
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

    pub fn translation(&self) -> Vector<f32> {
        self.pos
    }
}

impl Default for InstancePosition {
    fn default() -> Self {
        return Self::new(Isometry::new(Vector::default(), 0.0), Vector::new(1.0, 1.0));
    }
}

/// Buffer holding multiple [Positions](crate::Isometry) in form of [Matrices](crate::Matrix).
pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    instances: u64,
    instance_size: u64,
}

impl InstanceBuffer {
    pub fn new<D: bytemuck::NoUninit>(gpu: &Gpu, data: &[D]) -> Self {
        let instance_size = size_of::<D>() as u64;
        let data = bytemuck::cast_slice(data);
        let buffer_size = data.len() as u64;
        assert!(buffer_size % instance_size == 0);
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: data,
            });

        return Self {
            buffer,
            buffer_size: buffer_size,
            instances: buffer_size / instance_size,
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
            buffer_size: 0,
        };
    }

    pub fn write<D: bytemuck::NoUninit>(&mut self, gpu: &Gpu, data: &[D]) {
        self.write_offset(gpu, 0, data);
    }

    pub fn write_offset<D: bytemuck::NoUninit>(
        &mut self,
        gpu: &Gpu,
        instance_offset: u64,
        data: &[D],
    ) {
        let data = bytemuck::cast_slice(data);
        let new_size = instance_offset * self.instance_size + data.len() as u64;
        assert_eq!(data.len() as u64 % self.instance_size, 0);
        assert_eq!(size_of::<D>() as u64, self.instance_size);

        self.instances = new_size / self.instance_size;

        if new_size > self.buffer_size {
            self.buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("instance_buffer"),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    contents: data,
                });
        } else {
            gpu.queue
                .write_buffer(&self.buffer, instance_offset * self.instance_size, data);
        }
        self.buffer_size = new_size;
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.buffer_size)
    }

    pub fn buffer_capacity(&self) -> wgpu::BufferAddress {
        self.buffer.size()
    }

    pub fn instances(&self) -> InstanceIndices {
        InstanceIndices::new(0, self.instance_amount() as u32)
    }

    pub fn buffer_size(&self) -> wgpu::BufferAddress {
        return self.buffer_size;
    }

    pub fn instance_amount(&self) -> wgpu::BufferAddress {
        self.instances
    }

    pub fn instance_size(&self) -> wgpu::BufferAddress {
        self.instance_size
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
        InstanceIndices::new(self.index, self.index + 1)
    }
}

impl Into<InstanceIndices> for u32 {
    fn into(self) -> InstanceIndices {
        InstanceIndices::new(self, self + 1)
    }
}

impl Into<InstanceIndices> for Range<u32> {
    fn into(self) -> InstanceIndices {
        InstanceIndices::new(self.start, self.end)
    }
}

#[derive(Debug, Copy, Clone)]
/// Range of [InstanceIndex]
pub struct InstanceIndices {
    pub start: u32,
    pub end: u32,
}

impl InstanceIndices {
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn range(&self) -> Range<u32> {
        self.start..self.end
    }
}
