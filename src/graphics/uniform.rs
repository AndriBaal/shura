use crate::graphics::Gpu;
use std::marker::PhantomData;

pub trait Uniform {
    fn bind_group(&self) -> &wgpu::BindGroup;
}

#[derive(Debug)]
pub struct UniformData<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> UniformData<T> {
    pub fn new(gpu: &Gpu, data: T) -> Self {
        let shared_assets = gpu.shared_assets();
        Self::custom_layout(gpu, &shared_assets.single_uniform_layout, data)
    }

    pub fn camera(gpu: &Gpu, data: T) -> Self {
        let shared_assets = gpu.shared_assets();
        Self::custom_layout(gpu, &shared_assets.camera_layout, data)
    }

    pub fn custom(gpu: &Gpu, desc: &wgpu::BindGroupLayoutDescriptor, data: T) -> Self {
        let layout = gpu.device.create_bind_group_layout(desc);
        Self::custom_layout(gpu, &layout, data)
    }

    pub fn empty(gpu: &Gpu, layout: &wgpu::BindGroupLayout) -> Self {
        const BUFFER_ALIGNMENT: u64 = 16;
        let data_size = std::mem::size_of::<T>() as u64;
        let buffer_size = wgpu::util::align_to(data_size, BUFFER_ALIGNMENT);
        debug_assert!(
            buffer_size % BUFFER_ALIGNMENT == 0,
            "Unaligned buffer size: {}",
            buffer_size
        );
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
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
        }
    }

    pub fn custom_layout(gpu: &Gpu, layout: &wgpu::BindGroupLayout, data: T) -> Self {
        let mut uniform = Self::empty(gpu, layout);
        uniform.write(gpu, data);
        uniform
    }

    pub fn write(&mut self, gpu: &Gpu, data: T) {
        gpu.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
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
