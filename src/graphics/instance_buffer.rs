use std::{
    marker::PhantomData,
    mem::size_of,
    ops::{Deref, Range},
};
use wgpu::util::DeviceExt;

use crate::{
    entity::EntityGroupManager,
    graphics::{Asset, Color, Gpu, SpriteArrayIndex},
    math::{Isometry2, Isometry3, Matrix2, Matrix4, Rotation2, Vector2, Vector3, AABB},
};

pub type ColorInstance2D = Instance2D<Color>;
pub type PositionInstance2D = Instance2D<()>;
pub type SpriteInstance2D = Instance2D<()>;
pub type SpriteArrayInstance2D = Instance2D<SpriteArrayIndex>;
pub type SpriteCropInstance2D = Instance2D<SpriteAtlas>;
pub type SpriteArrayCropInstance2D = Instance2D<SpriteArrayAtlas>;

pub trait Instance: bytemuck::Pod + bytemuck::Zeroable + Send + Sync {
    const ATTRIBUTES: &'static [wgpu::VertexFormat];
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpriteAtlas {
    pub offset: Vector2<f32>,
    pub scaling: Vector2<f32>,
    pub alpha: f32,
}

impl SpriteAtlas {
    pub fn new(top_left: Vector2<f32>, bottom_right: Vector2<f32>, alpha: f32) -> Self {
        let aabb = AABB::new(top_left, bottom_right);
        let offset = aabb.center();
        let scaling = aabb.dim();
        Self {
            offset,
            scaling,
            alpha,
        }
    }
}

impl Default for SpriteAtlas {
    fn default() -> Self {
        Self {
            scaling: Vector2::new(1.0, 1.0),
            offset: Vector2::default(),
            alpha: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpriteArrayAtlas {
    pub offset: Vector2<f32>,
    pub scaling: Vector2<f32>,
    pub alpha: f32,
    pub index: SpriteArrayIndex,
}

impl SpriteArrayAtlas {
    pub fn new(
        top_left: Vector2<f32>,
        bottom_right: Vector2<f32>,
        alpha: f32,
        index: SpriteArrayIndex,
    ) -> Self {
        let aabb = AABB::new(top_left, bottom_right);
        let offset = aabb.center();
        let scaling = aabb.dim();
        Self {
            offset,
            scaling,
            alpha,
            index,
        }
    }
}

impl Default for SpriteArrayAtlas {
    fn default() -> Self {
        Self {
            scaling: Vector2::new(1.0, 1.0),
            offset: Vector2::default(),
            alpha: 1.0,
            index: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Instance2D<D: bytemuck::Pod> {
    pub translation: Vector2<f32>,
    pub scale_rotation: Matrix2<f32>,
    pub data: D,
}

unsafe impl<D: bytemuck::Pod> bytemuck::Pod for Instance2D<D> {}

impl Instance for ColorInstance2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x4,
    ];
}

impl Instance for SpriteArrayInstance2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Uint32,
    ];
}

impl Instance for SpriteCropInstance2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32,
    ];
}

impl Instance for SpriteArrayCropInstance2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32,
        wgpu::VertexFormat::Uint32,
    ];
}

impl Instance for PositionInstance2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] =
        &[wgpu::VertexFormat::Float32x2, wgpu::VertexFormat::Float32x4];
}

impl<D: bytemuck::Pod> Instance2D<D> {
    pub fn new(position: Isometry2<f32>, data: D) -> Self {
        Self {
            scale_rotation: Matrix2::new(
                position.rotation.cos_angle(),
                position.rotation.sin_angle(),
                -position.rotation.sin_angle(),
                position.rotation.cos_angle(),
            ),
            translation: position.translation.vector,
            data,
        }
    }

    pub fn with_scaling(position: Isometry2<f32>, scaling: Vector2<f32>, data: D) -> Self {
        Self {
            scale_rotation: Matrix2::new(
                scaling.x * position.rotation.cos_angle(),
                scaling.x * position.rotation.sin_angle(),
                scaling.y * -position.rotation.sin_angle(),
                scaling.y * position.rotation.cos_angle(),
            ),
            translation: position.translation.vector,
            data,
        }
    }

    pub fn set_data(&mut self, data: D) {
        self.data = data;
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.translation = translation;
    }

    pub fn set_scale_rotation(&mut self, scale: Vector2<f32>, rotation: Rotation2<f32>) {
        self.scale_rotation = Matrix2::new(scale.x, 0.0, 0.0, scale.y)
            * Matrix2::new(
                rotation.cos_angle(),
                -rotation.sin_angle(),
                rotation.sin_angle(),
                rotation.cos_angle(),
            )
    }
}

impl<D: bytemuck::Pod + Default> Default for Instance2D<D> {
    fn default() -> Self {
        Self::new(
            Isometry2::new(Vector2::default(), 0.0),
            Default::default(),
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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BufferCall {
    Manual,
    EveryFrame,
}

#[derive(Debug)]
pub struct InstanceBuffer<I: Instance> {
    buffer: wgpu::Buffer,
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

        if new_size > self.buffer_size() {
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

        self.instances = new_size / instance_size;
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..self.buffer_size())
    }

    pub fn buffer_capacity(&self) -> wgpu::BufferAddress {
        self.buffer.size()
    }

    pub fn instances(&self) -> Range<u32> {
        0..self.instance_amount() as u32
    }

    pub fn buffer_size(&self) -> wgpu::BufferAddress {
        I::SIZE * self.instance_amount() as u64
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

pub struct SmartInstanceBuffer<I: Instance> {
    pub call: BufferCall,
    pub buffer_on_group_change: bool,
    pub needs_update: bool,
    pub instances: Vec<I>,
    pub force_update: bool,
    buffer: Option<InstanceBuffer<I>>,
}

impl<I: Instance> SmartInstanceBuffer<I> {
    pub const MANUAL: SmartInstanceBuffer<I> = SmartInstanceBuffer {
        call: BufferCall::Manual,
        buffer_on_group_change: false,
        needs_update: true,
        instances: Vec::new(),
        force_update: true,
        buffer: None,
    };

    pub const GROUP_CHANGED: SmartInstanceBuffer<I> = SmartInstanceBuffer {
        call: BufferCall::Manual,
        buffer_on_group_change: true,
        needs_update: true,
        instances: Vec::new(),
        force_update: true,
        buffer: None,
    };

    pub const EVERY_FRAME: SmartInstanceBuffer<I> = SmartInstanceBuffer {
        call: BufferCall::EveryFrame,
        buffer_on_group_change: true,
        needs_update: true,
        instances: Vec::new(),
        force_update: true,
        buffer: None,
    };

    pub fn push(&mut self, instance: I) {
        self.instances.push(instance);
    }

    pub fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        self.instances.extend(iter);
    }

    pub fn buffer(&self) -> &InstanceBuffer<I> {
        self.buffer.as_ref().unwrap()
    }
}

impl<I: Instance> Default for SmartInstanceBuffer<I> {
    fn default() -> Self {
        Self::EVERY_FRAME
    }
}

impl<I: Instance> Asset for SmartInstanceBuffer<I> {
    fn needs_update(&self) -> bool {
        return self.needs_update;
    }

    fn prepare(&mut self, groups: &EntityGroupManager) {
        self.needs_update = self.call == BufferCall::EveryFrame
            || self.force_update
            || (groups.render_groups_changed() && self.buffer_on_group_change)
    }

    fn apply(&mut self, gpu: &Gpu) {
        if self.needs_update {
            if let Some(buffer) = self.buffer.as_mut() {
                buffer.write(gpu, &self.instances)
            } else {
                self.buffer = Some(gpu.create_instance_buffer(&self.instances))
            }
            self.instances.clear();
            self.force_update = false;
            self.needs_update = false;
        }
    }
}

impl<I: Instance> Deref for SmartInstanceBuffer<I> {
    type Target = InstanceBuffer<I>;
    fn deref(&self) -> &Self::Target {
        self.buffer()
    }
}
