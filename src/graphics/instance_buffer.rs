use std::{marker::PhantomData, mem::size_of, ops::Range};

use crate::{
    Color, Gpu, Isometry2, Isometry3, Matrix2, Matrix4, Rotation2, SpriteSheetIndex, Vector2,
    Vector3,
};
use wgpu::{util::DeviceExt, vertex_attr_array};

pub type InstanceBuffer2D = InstanceBuffer<Instance2D>;
pub type InstanceBuffer3D = InstanceBuffer<Instance3D>;

pub trait Instance: bytemuck::Pod + bytemuck::Zeroable {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute];
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
}

impl Instance for () {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[];
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

/// Single vertex of a mesh. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instance2D {
    pub pos: Vector2<f32>,
    pub rot: Matrix2<f32>,
    pub atlas: SpriteAtlas,
    pub color: Color,
    pub sprite_sheet_index: SpriteSheetIndex,
}

impl Instance for Instance2D {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &vertex_attr_array![
        2 => Float32x2,
        3 => Float32x4,
        4 => Float32x2,
        5 => Float32x2,
        6 => Float32x4,
        7 => Uint32,
    ];
}

impl Instance2D {
    pub fn new(
        pos: Isometry2<f32>,
        scaling: Vector2<f32>,
        atlas: SpriteAtlas,
        color: Color,
        sprite_sheet_index: SpriteSheetIndex,
    ) -> Self {
        Self {
            rot: Matrix2::new(
                scaling.x * pos.rotation.cos_angle(),
                scaling.x * pos.rotation.sin_angle(),
                scaling.y * -pos.rotation.sin_angle(),
                scaling.y * pos.rotation.cos_angle(),
            ),
            pos: pos.translation.vector,
            atlas,
            color,
            sprite_sheet_index,
        }
    }

    pub fn new_position(pos: Isometry2<f32>, scaling: Vector2<f32>) -> Self {
        Self {
            rot: Matrix2::new(
                scaling.x * pos.rotation.cos_angle(),
                scaling.x * pos.rotation.sin_angle(),
                scaling.y * -pos.rotation.sin_angle(),
                scaling.y * pos.rotation.cos_angle(),
            ),
            pos: pos.translation.vector,
            ..Default::default()
        }
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.pos = translation;
    }

    pub fn set_position(&mut self, pos: Isometry2<f32>, scaling: Vector2<f32>) {
        self.rot = Matrix2::new(
            scaling.x * pos.rotation.cos_angle(),
            scaling.x * pos.rotation.sin_angle(),
            scaling.y * -pos.rotation.sin_angle(),
            scaling.y * pos.rotation.cos_angle(),
        );
        self.pos = pos.translation.vector;
    }

    pub fn set_rotation_scaling(&mut self, scaling: Vector2<f32>, rotation: Rotation2<f32>) {
        self.rot = Matrix2::new(scaling.x, 0.0, 0.0, scaling.y)
            * Matrix2::new(
                rotation.cos_angle(),
                -rotation.sin_angle(),
                rotation.sin_angle(),
                rotation.cos_angle(),
            )
    }

    pub fn translation(&self) -> Vector2<f32> {
        self.pos
    }
}

impl Default for Instance2D {
    fn default() -> Self {
        return Self::new(
            Isometry2::new(Vector2::default(), 0.0),
            Vector2::new(1.0, 1.0),
            Default::default(),
            Color::WHITE,
            0,
        );
    }
}

/// Single vertex of a mesh. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instance3D {
    pub matrix: Matrix4<f32>,
}

impl Instance for Instance3D {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &vertex_attr_array![
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4,
    ];
}

impl Instance3D {
    pub fn new(position: Isometry3<f32>, scaling: Vector3<f32>) -> Self {
        let mut matrix = position.to_matrix();
        matrix.append_nonuniform_scaling_mut(&scaling);
        Self { matrix }
    }
}

impl Default for Instance3D {
    fn default() -> Self {
        return Self::new(Default::default(), Vector3::new(1.0, 1.0, 1.0));
    }
}

/// Buffer holding multiple [Positions](crate::Isometry2) in form of [Matrices](crate::Matrix2).
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
        assert!(buffer_size % instance_size == 0);
        assert!(I::SIZE != 0);
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
            marker: PhantomData,
        };
    }

    pub fn empty(gpu: &Gpu, amount: u64) -> Self {
        assert!(I::SIZE != 0);
        let instance_size = size_of::<I>() as u64;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: instance_size as u64 * amount as u64,
            mapped_at_creation: false,
        });

        return Self {
            buffer,
            instances: 0,
            buffer_size: 0,
            marker: PhantomData,
        };
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[I]) {
        self.write_offset(gpu, 0, data);
    }

    pub fn write_offset(&mut self, gpu: &Gpu, instance_offset: u64, data: &[I]) {
        let instance_size: u64 = I::SIZE as u64;
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
        } else {
            if !data.is_empty() {
                gpu.queue
                    .write_buffer(&self.buffer, instance_offset * instance_size, data);
            }
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
        I::SIZE as u64
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

#[derive(Debug, Copy, Clone)]
/// Index of a [Position](crate::Isometry2) in a [InstanceBuffer] represented by a [Matrix2]
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
