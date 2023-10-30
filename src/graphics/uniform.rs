use crate::Gpu;
use std::marker::PhantomData;

/// Uniform abstraction used to send data to the GPU. You can use the uniform in the
/// shader by binding it via the `Renderer`. You should look that every Uniform
/// is 16 byte aligned since not all gpu.devices / browser support other aligned values.
#[derive(Debug)]
pub struct Uniform<T: bytemuck::Pod> {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    marker: PhantomData<T>,
}

impl<T: bytemuck::Pod> Uniform<T> {
    pub fn new(gpu: &Gpu, data: T) -> Self {
        Self::new_custom_layout(&gpu, &gpu.base.single_uniform_layout, data)
    }

    pub fn new_vertex(gpu: &Gpu, data: T) -> Self {
        Self::new_custom_layout(&gpu, &gpu.base.camera_layout, data)
    }

    pub fn new_custom(gpu: &Gpu, desc: &wgpu::BindGroupLayoutDescriptor, data: T) -> Uniform<T> {
        let layout = gpu.device.create_bind_group_layout(desc);
        return Self::new_custom_layout(gpu, &layout, data);
    }

    pub fn new_custom_layout(gpu: &Gpu, layout: &wgpu::BindGroupLayout, data: T) -> Uniform<T> {
        const BUFFER_ALIGNMENT: u64 = 16;
        let data_size = std::mem::size_of_val(&data) as u64;
        let buffer_size = wgpu::util::align_to(data_size, BUFFER_ALIGNMENT);
        assert!(
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

        gpu.queue
            .write_buffer(&buffer, 0, bytemuck::cast_slice(&[data]));

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

    pub fn write(&mut self, gpu: &Gpu, data: T) {
        gpu.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }

    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
