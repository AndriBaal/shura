use crate::Gpu;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct Uniform<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> Uniform<T> {
    pub fn new(gpu: &Gpu, data: T) -> Self {
        Self::custom_layout(gpu, &gpu.base.single_uniform_layout, data)
    }

    pub fn camera(gpu: &Gpu, data: T) -> Self {
        Self::custom_layout(gpu, &gpu.base.camera_layout, data)
    }

    pub fn custom(gpu: &Gpu, desc: &wgpu::BindGroupLayoutDescriptor, data: T) -> Uniform<T> {
        let layout = gpu.device.create_bind_group_layout(desc);
        Self::custom_layout(gpu, &layout, data)
    }

    pub fn empty(gpu: &Gpu, layout: &wgpu::BindGroupLayout) -> Uniform<T> {
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

        Uniform {
            buffer,
            bind_group,
            marker: PhantomData::<T>,
        }
    }

    pub fn custom_layout(gpu: &Gpu, layout: &wgpu::BindGroupLayout, data: T) -> Uniform<T> {
        let mut uniform = Uniform::empty(gpu, layout);
        uniform.write(gpu, data);
        uniform
    }

    pub fn write(&mut self, gpu: &Gpu, data: T) {
        gpu.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
