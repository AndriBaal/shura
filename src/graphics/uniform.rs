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
        Self::new_custom(&gpu, &gpu.base.fragment_uniform, data)
    }

    pub(crate) fn new_vertex(gpu: &Gpu, data: T) -> Self {
        Self::new_custom(&gpu, &gpu.base.vertex_uniform, data)
    }

    pub(crate) fn new_custom(gpu: &Gpu, layout: &wgpu::BindGroupLayout, data: T) -> Uniform<T> {
        const BUFFER_ALIGNMENT: usize = 16;
        let data_size = std::mem::size_of_val(&data);
        let buffer_size =
            ((data_size + BUFFER_ALIGNMENT - 1) / BUFFER_ALIGNMENT) * BUFFER_ALIGNMENT;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: buffer_size as u64,
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
            label: Some("uniform_bindgroup"),
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
