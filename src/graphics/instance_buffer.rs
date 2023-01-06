use crate::{math::matrix::Matrix, Gpu};
use wgpu::util::DeviceExt;

/// Buffer holding multiple [Positions](crate::Isometry) in form of [Matrices](crate::Matrix).
pub struct InstanceBuffer {
    buffer: wgpu::Buffer,
}

impl InstanceBuffer {
    pub fn new(gpu: &Gpu, data: &[Matrix]) -> Self {
        Self::new_wgpu(&gpu.device, data)
    }

    pub(crate) fn new_wgpu(device: &wgpu::Device, data: &[Matrix]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance_buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(data),
        });

        return Self { buffer };
    }

    pub fn write(&mut self, gpu: &Gpu, data: &[Matrix]) {
        assert_eq!(data.len() as u32, self.instances());
        gpu.queue
            .write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }

    pub fn slice(&self) -> wgpu::BufferSlice {
        self.buffer.slice(..)
    }

    pub fn size(&self) -> u64 {
        self.buffer.size()
    }

    pub fn instances(&self) -> u32 {
        const MATRIX_SIZE: u64 = std::mem::size_of::<Matrix>() as u64;
        let buffer_size = self.size();
        return (buffer_size / MATRIX_SIZE) as u32;
    }
}
