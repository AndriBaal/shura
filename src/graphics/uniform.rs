use crate::graphics::Gpu;
use std::{marker::PhantomData, sync::Arc};

pub trait Uniform {
    fn bind_group(&self) -> &wgpu::BindGroup;
}

#[derive(Debug)]
pub struct UniformData<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    layout: Arc<wgpu::BindGroupLayout>,
    marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> UniformData<T> {
    pub const BUFFER_ALIGNMENT: u64 = 16;
    pub fn empty(gpu: &Gpu, layout: Arc<wgpu::BindGroupLayout>, amount: u32) -> Self {
        let data_size = std::mem::size_of::<T>() as u64 * amount as u64;
        let buffer_size = wgpu::util::align_to(data_size, Self::BUFFER_ALIGNMENT);
        debug_assert!(
            buffer_size % Self::BUFFER_ALIGNMENT == 0,
            "Unaligned buffer size: {}",
            buffer_size
        );
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        Self {
            buffer,
            bind_group,
            marker: PhantomData::<T>,
            layout,
        }
    }

    pub fn new(gpu: &Gpu, layout: Arc<wgpu::BindGroupLayout>, data: &[T]) -> Self {
        let mut uniform = Self::empty(gpu, layout, data.len() as u32);
        uniform.write(gpu, data);
        return uniform;
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[T]) {
        let bytes = bytemuck::cast_slice(data);
        let bytes_size = wgpu::util::align_to(bytes.len() as u64, Self::BUFFER_ALIGNMENT);
        let buffer_size = self.buffer.size();
        if bytes_size as u64 <= buffer_size {
            gpu.queue.write_buffer(&self.buffer, 0, bytes);
        } else {
            *self = Self::new(gpu, self.layout.clone(), data);
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

impl<T: bytemuck::Pod> Uniform for UniformData<T> {
    fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
