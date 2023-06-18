use std::sync::Mutex;

use glyph_brush::Rectangle;
use wgpu::util::DeviceExt;

use crate::{CameraMatrix, Gpu, Vector};

use super::text_pipeline::TextVertex;

pub(crate) struct TextCache {
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    // pub vertex_buffer_len: usize,
    pub vertices: Mutex<u32>,
    matrix_buffer: wgpu::Buffer,
}

impl TextCache {
    pub fn new(gpu: &Gpu, dim: Vector<u32>, max_chars: u64) -> Self {
        let size = wgpu::Extent3d {
            width: dim.x,
            height: dim.y,
            depth_or_array_layers: 1,
        };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu-text Cache Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let matrix_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("wgpu-text Matrix Buffer"),
                contents: bytemuck::cast_slice(&[CameraMatrix::ortho(dim.cast::<f32>())]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-text Bind Group"),
            layout: &gpu.base.text_pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&gpu.base.text_pipeline.sampler),
                },
            ],
        });

        let vertex_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wgpu-text Vertex Buffer"),
            size: max_chars * std::mem::size_of::<TextVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            texture,
            bind_group,
            matrix_buffer,
            vertex_buffer,
            // vertex_buffer_len: 0,
            vertices: Mutex::new(0),
        }
    }

    pub fn update_texture(&self, gpu: &Gpu, size: Rectangle<u32>, data: &[u8]) {
        gpu.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: size.min[0],
                    y: size.min[1],
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(size.width()),
                rows_per_image: Some(size.height()),
            },
            wgpu::Extent3d {
                width: size.width(),
                height: size.height(),
                depth_or_array_layers: 1,
            },
        )
    }

    pub fn update_matrix(&self, gpu: &Gpu, matrix: CameraMatrix) {
        gpu.queue
            .write_buffer(&self.matrix_buffer, 0, bytemuck::cast_slice(&[matrix]));
    }

    pub fn update_vertex_buffer(&self, gpu: &Gpu, vertices: Vec<TextVertex>) {
        let data: &[u8] = bytemuck::cast_slice(&vertices);
        *self.vertices.lock().unwrap() = vertices.len() as u32;
        gpu.queue.write_buffer(&self.vertex_buffer, 0, data);
    }

    pub fn vertices(&self) -> u32 {
        *self.vertices.lock().unwrap()
    }
}
