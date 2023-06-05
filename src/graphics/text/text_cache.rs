use glyph_brush::Rectangle;
use wgpu::util::DeviceExt;

use crate::{Gpu, Matrix, Vector};

use super::text_pipeline::TextVertex;

pub(crate) struct TextCache {
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_buffer_len: usize,
    pub vertices: u32,
    matrix_buffer: wgpu::Buffer,
}

impl TextCache {
    pub fn new(gpu: &Gpu, dim: Vector<u32>) -> Self {
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
                contents: bytemuck::cast_slice(&[Matrix::ortho(dim.cast::<f32>())]),
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
            size: 0,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            texture,
            bind_group,
            matrix_buffer,
            vertex_buffer,
            vertex_buffer_len: 0,
            vertices: 0,
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

    pub(crate) fn recreate_texture(&mut self, gpu: &Gpu, dim: Vector<u32>) {
        let pipeline = &gpu.base.text_pipeline;
        self.texture = Self::create_cache_texture(gpu, dim);
        self.bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wgpu-text Bind Group"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.matrix_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                },
            ],
        });
    }

    pub(crate) fn update_matrix(&self, gpu: &Gpu, matrix: Matrix) {
        gpu.queue
            .write_buffer(&self.matrix_buffer, 0, bytemuck::cast_slice(&[matrix]));
    }

    // TODO look into preallocating the vertex buffer instead of constantly reallocating
    pub(crate) fn update_vertex_buffer(&mut self, gpu: &Gpu, vertices: Vec<TextVertex>) {
        self.vertices = vertices.len() as u32;
        let data: &[u8] = bytemuck::cast_slice(&vertices);

        if vertices.len() > self.vertex_buffer_len {
            self.vertex_buffer_len = vertices.len();

            self.vertex_buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wgpu-text Vertex Buffer"),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    contents: data,
                });

            return;
        }
        gpu.queue.write_buffer(&self.vertex_buffer, 0, data);
    }

    pub(crate) fn create_cache_texture(gpu: &Gpu, dimensions: Vector<u32>) -> wgpu::Texture {
        let size = wgpu::Extent3d {
            width: dimensions.x,
            height: dimensions.y,
            depth_or_array_layers: 1,
        };
        gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wgpu-text Cache Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    }
}
