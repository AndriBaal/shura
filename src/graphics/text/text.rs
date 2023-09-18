use std::{mem::size_of, sync::Arc};

use crate::{vector, Color, Gpu, Index, SpriteSheet, SpriteSheetBuilder, SpriteSheetIndex, Vector};
use rustc_hash::FxHashMap;
use wgpu::util::DeviceExt;

pub use fontdue::layout::{LayoutSettings, HorizontalAlign, VerticalAlign};

#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
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
    index_map: FxHashMap<u16, SpriteSheetIndex>,
    font: fontdue::Font,
}
impl FontInner {
    pub fn new(gpu: &Gpu, data: &[u8]) -> Self {
        const RES: f32 = 200.0;
        let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default()).unwrap();
        let width = font
            .chars()
            .iter()
            .map(|(_, i)| font.metrics_indexed(i.get(), RES).width)
            .max()
            .unwrap() as u32;

        let amount = font
            .chars()
            .iter()
            .filter(|(_, i)| font.metrics_indexed(i.get(), RES).width > 0)
            .count();

        let desc = SpriteSheetBuilder::empty(vector(width, RES as u32), vector(amount as u32, 1))
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

        let mut counter = 0;
        let mut index_map = FxHashMap::default();
        for (_char, index) in font.chars() {
            let (metrics, data) = font.rasterize_indexed(index.get(), RES);
            if data.len() > 0 {
                index_map.insert(index.get(), counter);
                sprite_sheet.write(
                    gpu,
                    counter,
                    vector(metrics.width as u32, metrics.height as u32),
                    1,
                    &data,
                );
                counter += 1;
            }
        }

        return Self {
            sprite_sheet,
            index_map,
            font,
        };
    }
}

pub struct TextSection<'a> {
    pub color: Color,
    pub text: &'a str,
    pub size: f32,
    pub layout: LayoutSettings,
}

pub struct Text {
    font: Font,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    amount_of_vertices: u32,
    amount_of_indices: u32,
}

impl Text {
    pub fn new(gpu: &Gpu, font: &Font, sections: &[TextSection]) -> Self {
        use fontdue::layout::*;

        let mut vertices: Vec<TextVertex> = vec![];
        let mut indices: Vec<Index> = vec![];

        for section in sections {
            let mut layout = Layout::<()>::new(CoordinateSystem::PositiveYUp);
            layout.reset(&section.layout);
            layout.append(
                &[&font.inner.font],
                &TextStyle::new(section.text, section.size, 0),
            );

            for glyph in layout.glyphs() {
                let metrics = font
                    .inner
                    .font
                    .metrics_indexed(glyph.key.glyph_index, section.size);
                let id = font.inner.index_map[&glyph.key.glyph_index];
                let bottom_left = vector(glyph.x, glyph.y);
                let bottom_right = vector(glyph.x + glyph.width as f32, glyph.y);
                let top_left = vector(glyph.x, glyph.y + glyph.height as f32);
                let top_right = vector(
                    glyph.x + glyph.width as f32,
                    glyph.y + glyph.height as f32,
                );

                let base_index = vertices.len() as u32;
                let offset = vector(0.0, metrics.ymin as f32);


                vertices.extend([
                    TextVertex::new(top_left - offset, Vector::new(0.0, 0.0), section.color, id),
                    TextVertex::new(bottom_left - offset, Vector::new(0.0, 1.0), section.color, id),
                    TextVertex::new(bottom_right - offset, Vector::new(1.0, 1.0), section.color, id),
                    TextVertex::new(top_right - offset, Vector::new(1.0, 0.0), section.color, id),
                ]);
                indices.extend([
                    Index::new(base_index + 0, base_index + 1, base_index + 2),
                    Index::new(base_index + 2, base_index + 3, base_index + 0),
                ]);
            }
        }

        // let mut layout = Layout::new(CoordinateSystem::PositiveYUp);
        // // By default, layout is initialized with the default layout settings. This call is redundant, but
        // // demonstrates setting the value with your custom settings.
        // layout.reset(&LayoutSettings {
        //     ..LayoutSettings::default()
        // });

        // let vertices = &[
        //     TextVertex::new(Vector::new(-1.0, 1.0), Vector::new(0.0, 0.0), Color::RED, 0),
        //     TextVertex::new(
        //         Vector::new(-1.0, -1.0),
        //         Vector::new(0.0, 1.0),
        //         Color::RED,
        //         0,
        //     ),
        //     TextVertex::new(Vector::new(1.0, -1.0), Vector::new(1.0, 1.0), Color::RED, 0),
        //     TextVertex::new(Vector::new(1.0, 1.0), Vector::new(1.0, 0.0), Color::RED, 0),
        //     TextVertex::new(
        //         Vector::new(-1.0 + 2.0, 1.0),
        //         Vector::new(0.0, 0.0),
        //         Color::YELLOW,
        //         0,
        //     ),
        //     TextVertex::new(
        //         Vector::new(-1.0 + 2.0, -1.0),
        //         Vector::new(0.0, 1.0),
        //         Color::YELLOW,
        //         0,
        //     ),
        //     TextVertex::new(
        //         Vector::new(1.0 + 2.0, -1.0),
        //         Vector::new(1.0, 1.0),
        //         Color::YELLOW,
        //         0,
        //     ),
        //     TextVertex::new(
        //         Vector::new(1.0 + 2.0, 1.0),
        //         Vector::new(1.0, 0.0),
        //         Color::YELLOW,
        //         0,
        //     ),
        // ];
        // let indices = &[
        //     Index::new(0, 1, 2),
        //     Index::new(2, 3, 0),
        //     Index::new(4, 5, 6),
        //     Index::new(6, 7, 4),
        // ];

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: bytemuck::cast_slice(&indices),
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

    pub fn vertex_buffer(&self) -> wgpu::BufferSlice {
        self.vertex_buffer.slice(..self.vertex_buffer.size())
    }

    pub fn index_buffer(&self) -> wgpu::BufferSlice {
        self.index_buffer
            .slice(..self.amount_of_indices as u64 * 3 * std::mem::size_of::<u32>() as u64)
    }

    pub fn amount_of_indices(&self) -> u32 {
        self.amount_of_indices * 3
    }

    pub fn amount_of_vertices(&self) -> u32 {
        self.amount_of_vertices
    }
}
