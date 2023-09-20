use std::{mem::size_of, sync::Arc};

use crate::{vector, Color, Gpu, Index, SpriteSheet, SpriteSheetBuilder, SpriteSheetIndex, Vector, Sprite};
use rustc_hash::FxHashMap;
use wgpu::util::DeviceExt;

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
    pub fn new(gpu: &Gpu, data: &'static [u8]) -> Self {
        let inner = FontInner::new(gpu, data);
        return Self {
            inner: Arc::new(inner),
        };
    }
}

pub(crate) struct FontInner {
    sprite_sheet: SpriteSheet,
    index_map: FxHashMap<rusttype::GlyphId, (SpriteSheetIndex, Vector<f32>)>,
    font: rusttype::Font<'static>,
}
impl FontInner {
    const RES: f32 = 200.0;

    pub fn new(gpu: &Gpu, data: &'static [u8]) -> Self {
        let scale = rusttype::Scale::uniform(Self::RES);
        let font = rusttype::Font::try_from_bytes(data).unwrap();

        let face_ref = match &font {
            rusttype::Font::Ref(r) => r,
            rusttype::Font::Owned(_) => unreachable!(),
        };

        macro_rules! glyphs {
            ($face_ref: expr) => {{
                let mut used_indices = std::collections::BTreeSet::new();
                $face_ref
                    .tables()
                    .cmap
                    .iter()
                    .flat_map(|c| c.subtables)
                    .filter(|s| s.is_unicode())
                    .flat_map(move |subtable| {
                        let mut pairs = Vec::new();
                        subtable.codepoints(|c| {
                            if let Ok(ch) = char::try_from(c) {
                                if let Some(idx) = subtable.glyph_index(c).filter(|i| i.0 > 0) {
                                    if used_indices.insert(idx.0) {
                                        pairs.push((rusttype::GlyphId(idx.0), ch));
                                    }
                                }
                            }
                        });
                        pairs
                    })
            }};
        }

        let mut amount = 0;
        let mut size = Vector::default();
        let glyphs = glyphs!(face_ref);
        for (id, _char) in glyphs {
            let glyph = font.glyph(id);
            let scaled = glyph.scaled(scale);
            let positioned = scaled.positioned(rusttype::Point { x: 0.0, y: 0.0 });

            if let Some(bb) = positioned.pixel_bounding_box() {
                amount += 1;
                if bb.width() > size.x {
                    size.x = bb.width();
                }
                if bb.height() > size.y {
                    size.y = bb.height();
                }
            }
        }

        let desc = SpriteSheetBuilder::empty(
            vector(amount, Self::RES as u32),
            vector(font.glyph_count() as u32, 1),
        )
        .sampler(Sprite::DEFAULT_SAMPLER)
        .format(wgpu::TextureFormat::R8Unorm);

        let mut sprite_sheet = gpu.create_sprite_sheet(desc);
        let mut index_map = FxHashMap::default();

        let glyphs = glyphs!(face_ref);
        let mut buffer: Vec<u8> = Vec::with_capacity((size.x * size.y) as usize);
        let mut counter = 0;
        for (id, _char) in glyphs {
            let glyph = font.glyph(id);
            let scaled = glyph.scaled(scale);
            let positioned = scaled.positioned(rusttype::Point { x: 0.0, y: 0.0 });

            if let Some(bb) = positioned.pixel_bounding_box() {
                positioned.draw(|_x, _y, a| {
                    buffer.push((a.round() * 255.0) as u8);
                });
                let ratio = Vector::new(bb.width(), bb.height())
                    .cast::<f32>()
                    .component_div(&size.cast::<f32>());
                index_map.insert(id, (counter, ratio));
                sprite_sheet.write(
                    gpu,
                    counter,
                    vector(bb.width() as u32, bb.height() as u32),
                    1,
                    &buffer,
                );
                buffer.clear();
                counter += 1;
            }
        }
        // for (_char, index) in font.chars() {
        //     let (metrics, data) = font.rasterize_indexed(index.get(), RES);
        //     if data.len() > 0 {
        //         index_map.insert(index.get(), counter);
        //         sprite_sheet.write(
        //             gpu,
        //             counter,
        //             vector(metrics.width as u32, metrics.height as u32),
        //             1,
        //             &data,
        //         );
        //         counter += 1;
        //     }
        // }

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
    pub offset: Vector<f32>,
    // pub width: f32,
}

pub struct Text {
    font: Font,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    amount_of_vertices: u32,
    amount_of_indices: u32,
    vertices_size: wgpu::BufferAddress,
    indices_size: wgpu::BufferAddress,
}

impl Text {
    pub fn new(gpu: &Gpu, font: &Font, sections: &[TextSection]) -> Self {
        let mut vertices: Vec<TextVertex> = vec![];
        let mut indices: Vec<Index> = vec![];

        for section in sections {
            for glyph in font.inner.font.layout(
                section.text,
                rusttype::Scale::uniform(section.size),
                rusttype::Point {
                    x: section.offset.x,
                    y: section.offset.y,
                },
            ) {
                if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                    let base_index = vertices.len() as u32;
                    let (id, scale) = font.inner.index_map[&glyph.id()];
                    let size = Vector::new(bb.width(), bb.height());
                    let bottom_left = Vector::new(glyph.position().x, glyph.position().y);
                    let top_right = bottom_left + size;
                    let bottom_right = bottom_left + Vector::new(size.x, 0.0);
                    let top_left = bottom_left + Vector::new(0.0, size.y);
                    let offset = Vector::new(0.0, -bb.max.y);

                    vertices.extend([
                        TextVertex::new(
                            top_left + offset,
                            Vector::new(0.0, 0.0),
                            section.color,
                            id,
                        ),
                        TextVertex::new(
                            bottom_left + offset,
                            Vector::new(0.0, scale.y),
                            section.color,
                            id,
                        ),
                        TextVertex::new(
                            bottom_right + offset,
                            Vector::new(scale.x, scale.y),
                            section.color,
                            id,
                        ),
                        TextVertex::new(
                            top_right + offset,
                            Vector::new(scale.x, 0.0),
                            section.color,
                            id,
                        ),
                    ]);
                    indices.extend([
                        Index::new(base_index + 0, base_index + 1, base_index + 2),
                        Index::new(base_index + 2, base_index + 3, base_index + 0),
                    ]);
                }
            }
        }

        // let mut layout = Layout::new(CoordinateSystem::PositiveYUp);
        // // By default, layout is initialized with the default layout settings. This call is redundant, but
        // // demonstrates setting the value with your custom settings.
        // layout.reset(&LayoutSettings {
        //     ..LayoutSettings::default()
        // });

        // let test = *font.inner.index_map.get(&'g').unwrap();

        // let vertices = [
        //     TextVertex::new(Vector::new(-1.0, 1.0), Vector::new(0.0, 0.0), Color::RED, test),
        //     TextVertex::new(
        //         Vector::new(-1.0, -1.0),
        //         Vector::new(0.0, 1.0),
        //         Color::RED,
        //         test,
        //     ),
        //     TextVertex::new(Vector::new(1.0, -1.0), Vector::new(1.0, 1.0), Color::RED, test),
        //     TextVertex::new(Vector::new(1.0, 1.0), Vector::new(1.0, 0.0), Color::RED, test),
        //     TextVertex::new(
        //         Vector::new(-1.0 + 2.0, 1.0),
        //         Vector::new(0.0, 0.0),
        //         Color::YELLOW,
        //         test,
        //     ),
        //     TextVertex::new(
        //         Vector::new(-1.0 + 2.0, -1.0),
        //         Vector::new(0.0, 1.0),
        //         Color::YELLOW,
        //         test,
        //     ),
        //     TextVertex::new(
        //         Vector::new(1.0 + 2.0, -1.0),
        //         Vector::new(1.0, 1.0),
        //         Color::YELLOW,
        //         test,
        //     ),
        //     TextVertex::new(
        //         Vector::new(1.0 + 2.0, 1.0),
        //         Vector::new(1.0, 0.0),
        //         Color::YELLOW,
        //         test,
        //     ),
        // ];
        // let indices = [
        //     Index::new(0, 1, 2),
        //     Index::new(2, 3, 0),
        //     Index::new(4, 5, 6),
        //     Index::new(6, 7, 4),
        // ];

        let vertices_slice = bytemuck::cast_slice(&vertices[..]);
        let indices_slice = bytemuck::cast_slice(&indices[..]);
        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: vertices_slice,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: indices_slice,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

        return Self {
            vertex_buffer,
            index_buffer,
            amount_of_vertices: vertices.len() as u32,
            amount_of_indices: indices.len() as u32 * 3,
            font: font.clone(),
            vertices_size: vertices_slice.len() as wgpu::BufferAddress,
            indices_size: indices_slice.len() as wgpu::BufferAddress,
        };
    }

    pub fn font(&self) -> &SpriteSheet {
        &self.font.inner.sprite_sheet
    }

    pub fn vertex_buffer(&self) -> wgpu::BufferSlice {
        self.vertex_buffer.slice(..self.vertices_size)
    }

    pub fn index_buffer(&self) -> wgpu::BufferSlice {
        self.index_buffer.slice(..self.indices_size)
    }

    pub fn amount_of_indices(&self) -> u32 {
        self.amount_of_indices
    }

    pub fn amount_of_vertices(&self) -> u32 {
        self.amount_of_vertices
    }

    pub fn vertices_size(&self) -> wgpu::BufferAddress {
        self.vertices_size
    }

    pub fn indices_size(&self) -> wgpu::BufferAddress {
        self.indices_size * 3
    }
}
