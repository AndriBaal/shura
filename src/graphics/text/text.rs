use std::sync::Arc;

use crate::{
    vector, Color, Gpu, Index, Isometry, Model, ModelBuilder, SpriteSheet, SpriteSheetBuilder,
    SpriteSheetIndex, Vector, Vertex,
};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug, Default, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub(crate) struct TextVertexData {
    color: Color,
    sprite: u32,
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
    const RES: f32 = 400.0;

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
            if !_char.is_ascii() {
                continue;
            }
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
            vector(size.x as u32, Self::RES as u32),
            vector(amount as u32, 1),
        )
        .sampler(wgpu::SamplerDescriptor {
            label: Some("wgpu-text Cache Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        })
        .format(wgpu::TextureFormat::R8Unorm);

        let mut sprite_sheet = gpu.create_sprite_sheet(desc);
        let mut index_map = FxHashMap::default();

        let glyphs = glyphs!(face_ref);
        let mut buffer: Vec<u8> = Vec::with_capacity((size.x * size.y) as usize);
        let mut counter = 0;
        for (id, _char) in glyphs {
            if !_char.is_ascii() {
                continue;
            }

            let glyph = font.glyph(id);
            let scaled = glyph.scaled(scale);
            let positioned = scaled.positioned(rusttype::Point { x: 0.0, y: 0.0 });

            if let Some(bb) = positioned.pixel_bounding_box() {
                positioned.draw(|_x, _y, a| {
                    buffer.push((a * 255.0) as u8);
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

        return Self {
            sprite_sheet,
            index_map,
            font,
        };
    }
}

pub enum TextAlignment {
    Start,
    Center,
    End,
}

pub struct TextSection<S: AsRef<str>> {
    pub color: Color,
    pub text: S,
    pub size: f32,
    pub vertex_offset: Isometry<f32>,
    pub vertex_rotation_axis: Vector<f32>,
    pub horizontal_alignment: TextAlignment,
    pub vertical_alignment: TextAlignment,
}

impl Default for TextSection<&str> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: "",
            size: 1.0,
            vertex_offset: Isometry::new(
                ModelBuilder::<()>::DEFAULT_OFFSET,
                ModelBuilder::<()>::DEFAULT_ROTATION,
            ),
            vertex_rotation_axis: Vector::new(0.0, 0.0),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
        }
    }
}

impl Default for TextSection<String> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: String::from(""),
            size: 1.0,
            vertex_offset: Isometry::new(
                ModelBuilder::<()>::DEFAULT_OFFSET,
                ModelBuilder::<()>::DEFAULT_ROTATION,
            ),
            vertex_rotation_axis: Vector::new(0.0, 0.0),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
        }
    }
}

pub struct Text {
    font: Font,
    model: Model,
}

impl Text {
    pub fn new<S: AsRef<str>>(gpu: &Gpu, font: &Font, sections: &[TextSection<S>]) -> Self {
        let builder = Self::compute_vertices(font, sections);
        let model = gpu.create_model_with_data(builder);
        return Self {
            font: font.clone(),
            model,
        };
    }

    fn compute_vertices<S: AsRef<str>>(
        font: &Font,
        sections: &[TextSection<S>],
    ) -> ModelBuilder<TextVertexData> {
        let mut vertices: Vec<Vertex<TextVertexData>> = vec![];
        let mut indices: Vec<Index> = vec![];

        fn compute_modifed_vertices(
            vertices: &mut [Vertex<TextVertexData>],
            vertex_offset: Isometry<f32>,
            vertex_rotation_axis: Vector<f32>,
        ) {
            let angle = vertex_offset.rotation.angle();
            if angle != ModelBuilder::<TextVertexData>::DEFAULT_ROTATION {
                for v in vertices.iter_mut() {
                    let delta = v.pos - vertex_rotation_axis;
                    v.pos = vertex_rotation_axis + vertex_offset.rotation * delta;
                }
            }

            if vertex_offset.translation.vector != ModelBuilder::<TextVertexData>::DEFAULT_OFFSET {
                for v in vertices.iter_mut() {
                    v.pos += vertex_offset.translation.vector;
                }
            }
        }
        for section in sections {
            let text = section.text.as_ref();
            if text.is_empty() {
                continue;
            }

            let scale = rusttype::Scale::uniform(section.size);
            let metrics = font.inner.font.v_metrics(scale);
            let mut off_y = 0.0;
            for line in text.lines() {
                let glyphs = font
                    .inner
                    .font
                    .layout(line, scale, rusttype::Point::default())
                    .collect::<Vec<rusttype::PositionedGlyph>>();

                let horizontal = match section.horizontal_alignment {
                    TextAlignment::Start => 0.0,
                    TextAlignment::Center => {
                        let mut max = 0.0;
                        for glyph in glyphs.iter().rev() {
                            if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                                max = glyph.position().x + bb.max.x;
                                break;
                            }
                        }
                        max / 2.0
                    }
                    TextAlignment::End => {
                        let mut max = 0.0;
                        for glyph in glyphs.iter().rev() {
                            if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                                max = glyph.position().x + bb.max.x;
                                break;
                            }
                        }
                        max
                    }
                };
                let vertical = match section.vertical_alignment {
                    TextAlignment::Start => 0.0,
                    TextAlignment::Center => section.size / 2.0,
                    TextAlignment::End => section.size,
                };

                for glyph in &glyphs {
                    if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                        let base_index = vertices.len() as u32;
                        if let Some((id, scale)) = font.inner.index_map.get(&glyph.id()) {
                            let size = Vector::new(bb.width(), bb.height());
                            let bottom_left = Vector::new(glyph.position().x, glyph.position().y);
                            let top_right = bottom_left + size;
                            let bottom_right = bottom_left + Vector::new(size.x, 0.0);
                            let top_left = bottom_left + Vector::new(0.0, size.y);
                            let offset = Vector::new(-horizontal, -bb.max.y - vertical - off_y);

                            vertices.extend([
                                Vertex::new(
                                    top_left + offset,
                                    Vector::new(0.0, 0.0),
                                    TextVertexData {
                                        color: section.color,
                                        sprite: *id,
                                    },
                                ),
                                Vertex::new(
                                    bottom_left + offset,
                                    Vector::new(0.0, scale.y),
                                    TextVertexData {
                                        color: section.color,
                                        sprite: *id,
                                    },
                                ),
                                Vertex::new(
                                    bottom_right + offset,
                                    Vector::new(scale.x, scale.y),
                                    TextVertexData {
                                        color: section.color,
                                        sprite: *id,
                                    },
                                ),
                                Vertex::new(
                                    top_right + offset,
                                    Vector::new(scale.x, 0.0),
                                    TextVertexData {
                                        color: section.color,
                                        sprite: *id,
                                    },
                                ),
                            ]);
                            let offset = vertices.len() - 4;
                            compute_modifed_vertices(
                                &mut vertices[offset..],
                                section.vertex_offset,
                                section.vertex_rotation_axis,
                            );
                            indices.extend([
                                Index::new(base_index + 0, base_index + 1, base_index + 2),
                                Index::new(base_index + 2, base_index + 3, base_index + 0),
                            ]);
                        }
                    }
                }

                off_y += section.size + metrics.descent.abs() + metrics.line_gap;
            }
        }
        return ModelBuilder::custom(vertices, indices);
    }

    pub fn write<S: AsRef<str>>(&mut self, gpu: &Gpu, sections: &[TextSection<S>]) {
        let builder = Self::compute_vertices(&self.font, sections);
        self.model.write_with_data(gpu, builder);
    }

    pub fn font(&self) -> &SpriteSheet {
        &self.font.inner.sprite_sheet
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}
