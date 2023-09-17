use std::{mem::size_of, sync::Arc};

use crate::{vector, Color, Gpu, Index, SpriteSheet, SpriteSheetBuilder, Vector};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub(crate) struct TextVertex {
    offset: Vector<f32>,
    tex: Vector<f32>,
    color: Color,
    sprite: u32,
}

impl TextVertex {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
            offset: size_of::<[f32; 2]>() as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
            offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
            shader_location: 2,
            format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
            offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
            shader_location: 3,
            format: wgpu::VertexFormat::Uint32,
        },
    ];
    pub const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBUTES,
    };

    pub fn new(offset: Vector<f32>, tex: Vector<f32>, color: Color, sprite: u32) -> Self {
        Self {
            offset,
            tex,
            color,
            sprite,
        }
    }
}

#[derive(Clone)]
pub struct Font {
    inner: Arc<FontInner>,
}

impl Font {
    pub fn new(gpu: &Gpu, data: &[u8]) -> Self {
        let inner = FontInner::new(gpu, data);
        return Self {
            inner: Arc::new(inner),
        };
    }
}

pub(crate) struct FontInner {
    sprite_sheet: SpriteSheet,
}
impl FontInner {
    pub fn new(gpu: &Gpu, data: &[u8]) -> Self {
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default()).unwrap();
        let res = 200;
        let width = font
            .chars()
            .iter()
            .map(|(_, i)| font.metrics_indexed(i.get(), res as f32).width)
            .max()
            .unwrap() as u32;

        let desc =
            SpriteSheetBuilder::empty(vector(width, res), vector(font.glyph_count() as u32, 0))
                .sampler(wgpu::SamplerDescriptor {
                    address_mode_u: wgpu::AddressMode::ClampToEdge,
                    address_mode_v: wgpu::AddressMode::ClampToEdge,
                    address_mode_w: wgpu::AddressMode::ClampToEdge,
                    mag_filter: wgpu::FilterMode::Linear,
                    min_filter: wgpu::FilterMode::Linear,
                    mipmap_filter: wgpu::FilterMode::Linear,
                    ..Default::default()
                })
                .format(wgpu::TextureFormat::R8Unorm);
        let mut sprite_sheet = gpu.create_sprite_sheet(desc);

        let (metrics, data) = font.rasterize('W', res as f32);
        sprite_sheet.write(
            gpu,
            0,
            vector(metrics.width as u32, metrics.height as u32),
            1,
            &data,
        );

        return Self { sprite_sheet };
    }
}

pub struct Text {
    font: Font,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    amount_of_vertices: u32,
    amount_of_indices: u32,
}

impl Text {
    pub fn new(gpu: &Gpu, font: &Font, text: &str) -> Self {
        let vertices = &[
            TextVertex::new(Vector::new(-1.0, 1.0), Vector::new(0.0, 0.0), Color::RED, 0),
            TextVertex::new(
                Vector::new(-1.0, -1.0),
                Vector::new(0.0, 1.0),
                Color::RED,
                0,
            ),
            TextVertex::new(Vector::new(1.0, -1.0), Vector::new(1.0, 1.0), Color::RED, 0),
            TextVertex::new(Vector::new(1.0, 1.0), Vector::new(1.0, 0.0), Color::RED, 0),
            TextVertex::new(
                Vector::new(-1.0 + 2.0, 1.0),
                Vector::new(0.0, 0.0),
                Color::YELLOW,
                0,
            ),
            TextVertex::new(
                Vector::new(-1.0 + 2.0, -1.0),
                Vector::new(0.0, 1.0),
                Color::YELLOW,
                0,
            ),
            TextVertex::new(
                Vector::new(1.0 + 2.0, -1.0),
                Vector::new(1.0, 1.0),
                Color::YELLOW,
                0,
            ),
            TextVertex::new(
                Vector::new(1.0 + 2.0, 1.0),
                Vector::new(1.0, 0.0),
                Color::YELLOW,
                0,
            ),
        ];
        let indices = &[
            Index::new(0, 1, 2),
            Index::new(2, 3, 0),
            Index::new(4, 5, 6),
            Index::new(6, 7, 4),
        ];

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

        return Self {
            vertex_buffer,
            index_buffer,
            amount_of_vertices: vertices.len() as u32,
            amount_of_indices: indices.len() as u32,
            font: font.clone(),
        };
    }

    pub fn font(&self) -> &SpriteSheet {
        &self.font.inner.sprite_sheet
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn amount_of_indices(&self) -> u32 {
        self.amount_of_indices * 3
    }

    pub fn amount_of_vertices(&self) -> u32 {
        self.amount_of_vertices
    }
}
