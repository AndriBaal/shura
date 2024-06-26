use std::{marker::PhantomData, mem::size_of, ops::Range};
use nalgebra::Isometry2;
use wgpu::util::DeviceExt;

use crate::{
    graphics::{Color, Gpu, SpriteArrayIndex},
    math::{Isometry3, Matrix4, Vector2, Vector3},
};

pub type InstanceBuffer2D = InstanceBuffer<Instance2D>;
pub type InstanceBuffer3D = InstanceBuffer<Instance3D>;

pub trait Instance: bytemuck::Pod + bytemuck::Zeroable {
    const ATTRIBUTES: &'static [wgpu::VertexFormat];
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
}

impl Instance for () {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[];
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpriteAtlas {
    pub offset: Vector2<f32>,
    pub scaling: Vector2<f32>,
}

impl SpriteAtlas {
    pub fn new(scaling: Vector2<f32>, offset: Vector2<f32>) -> Self {
        Self { offset, scaling }
    }
}

impl Default for SpriteAtlas {
    fn default() -> Self {
        Self {
            scaling: Vector2::new(1.0, 1.0),
            offset: Vector2::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instance2D {
    pub translation: Vector2<f32>,
    pub scaling: Vector2<f32>,
    pub rotation: f32,
    pub atlas: SpriteAtlas,
    pub color: Color,
    pub sprite_array_index: SpriteArrayIndex,
}

impl Instance for Instance2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Uint32,
    ];
}

impl Instance2D {
    pub fn new(
        translation: Vector2<f32>,
        rotation: f32,
        scaling: Vector2<f32>,
        atlas: SpriteAtlas,
        color: Color,
        sprite_array_index: SpriteArrayIndex,
    ) -> Self {
        Self {
            rotation,
            translation,
            atlas,
            color,
            sprite_array_index,
            scaling,
        }
    }

    pub fn set_position(&mut self, pos: Isometry2<f32>) {
        self.rotation = pos.rotation.angle();
        self.translation = pos.translation.vector;
    }

    pub fn position(&self) -> Isometry2<f32> {
        Isometry2::new(self.translation, self.rotation)
    }
}

impl Default for Instance2D {
    fn default() -> Self {
        Self::new(
            Vector2::default(),
            0.0,
            Vector2::new(1.0, 1.0),
            Default::default(),
            Color::WHITE,
            0,
        )
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instance3D {
    pub matrix: Matrix4<f32>,
}

impl Instance for Instance3D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x4,
    ];
}

impl Instance3D {
    pub fn new(position: Isometry3<f32>, scaling: Vector3<f32>) -> Self {
        let mut matrix = position.to_matrix();
        matrix.prepend_nonuniform_scaling_mut(&scaling);
        Self { matrix }
    }
}

impl Default for Instance3D {
    fn default() -> Self {
        Self::new(Default::default(), Vector3::new(1.0, 1.0, 1.0))
    }
}

#[derive(Debug)]
pub struct InstanceBuffer<I: Instance> {
    buffer: wgpu::Buffer,
    buffer_size: wgpu::BufferAddress,
    instances: u64,
    marker: PhantomData<I>,
}

impl<I: Instance> InstanceBuffer<I> {
    pub fn new(gpu: &Gpu, data: &[I]) -> Self {
        let instance_size = size_of::<I>() as u64;
        let data = bytemuck::cast_slice(data);
        let buffer_size = data.len() as u64;
        debug_assert!(buffer_size % instance_size == 0);
        debug_assert!(I::SIZE != 0);
        let buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("instance_buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: data,
            });

        Self {
            buffer,
            buffer_size,
            instances: buffer_size / instance_size,
            marker: PhantomData,
        }
    }

    pub fn empty(gpu: &Gpu, amount: u64) -> Self {
        debug_assert!(I::SIZE != 0);
        let instance_size = size_of::<I>() as u64;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: instance_size * amount,
            mapped_at_creation: false,
        });

        Self {
            buffer,
            instances: 0,
            buffer_size: 0,
            marker: PhantomData,
        }
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[I]) {
        self.write_offset(gpu, 0, data);
    }

    pub fn write_offset(&mut self, gpu: &Gpu, instance_offset: u64, data: &[I]) {
        let instance_size: u64 = I::SIZE;
        let data = bytemuck::cast_slice(data);
        let new_size = instance_offset * instance_size + data.len() as u64;

        if new_size > self.buffer_size {
            self.buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("instance_buffer"),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    contents: data,
                });
        } else if !data.is_empty() {
            gpu.queue
                .write_buffer(&self.buffer, instance_offset * instance_size, data);
        }

        self.buffer_size = new_size;
        self.instances = new_size / instance_size;
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.buffer_size)
    }

    pub fn buffer_capacity(&self) -> wgpu::BufferAddress {
        self.buffer.size()
    }

    pub fn instances(&self) -> Range<u32> {
        0..self.instance_amount() as u32
    }

    pub fn buffer_size(&self) -> wgpu::BufferAddress {
        self.buffer_size
    }

    pub fn instance_amount(&self) -> wgpu::BufferAddress {
        self.instances
    }

    pub fn instance_size(&self) -> wgpu::BufferAddress {
        I::SIZE
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

// #[derive(Debug, Copy, Clone)]
// pub struct InstanceIndex {
//     pub index: u32,
// }

// impl InstanceIndex {
//     pub const fn new(index: u32) -> Self {
//         Self { index }
//     }
// }

// impl From<InstanceIndex> for InstanceIndices {
//     fn from(val: InstanceIndex) -> Self {
//         InstanceIndices::new(val.index, val.index + 1)
//     }
// }

// impl From<u32> for InstanceIndices {
//     fn from(val: u32) -> Self {
//         InstanceIndices::new(val, val + 1)
//     }
// }

// impl From<Range<u32>> for InstanceIndices {
//     fn from(val: Range<u32>) -> Self {
//         InstanceIndices::new(val.start, val.end)
//     }
// }

// impl<I: Instance> From<&InstanceBuffer<I>> for InstanceIndices {
//     fn from(value: &InstanceBuffer<I>) -> Self {
//         value.instances()
//     }
// }

// #[derive(Debug, Copy, Clone)]
// pub struct InstanceIndices {
//     pub start: u32,
//     pub end: u32,
// }

// impl InstanceIndices {
//     pub const fn new(start: u32, end: u32) -> Self {
//         Self { start, end }
//     }

//     pub fn range(&self) -> Range<u32> {
//         self.start..self.end
//     }
// }
