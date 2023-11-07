use crate::{
    load_bytes, Color, Gpu, Index, Isometry2, Mesh, MeshBuilder2D, SpriteSheet,
    SpriteSheetBuilder, SpriteSheetIndex, Vector2, Vertex,
};
use owned_ttf_parser::AsFaceRef;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use wgpu::vertex_attr_array;

pub enum FontBuilder {
    Ref(&'static [u8]),
    Owned(Vec<u8>),
}

impl<'a> FontBuilder {
    pub fn bytes(bytes: &'static [u8]) -> Self {
        Self::Ref(bytes)
    }

    pub fn owned(bytes: Vec<u8>) -> Self {
        Self::Owned(bytes)
    }

    pub async fn file(path: &str) -> Self {
        let bytes = load_bytes(path).await.unwrap();
        Self::Owned(bytes)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex2DText {
    pub pos: Vector2<f32>,
    pub tex: Vector2<f32>,
    pub color: Color,
    pub sprite: u32,
}

impl Vertex for Vertex2DText {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Uint32,
    ];
}

#[derive(Clone)]
pub struct Font {
    inner: Arc<FontInner>,
}

impl Font {
    pub fn new(gpu: &Gpu, builder: FontBuilder) -> Self {
        let inner = FontInner::new(gpu, builder);
        return Self {
            inner: Arc::new(inner),
        };
    }
}

pub(crate) struct FontInner {
    sprite_sheet: SpriteSheet,
    index_map: FxHashMap<rusttype::GlyphId, (SpriteSheetIndex, Vector2<f32>)>,
    font: rusttype::Font<'static>,
}
impl FontInner {
    const RES: f32 = 400.0;

    pub fn new(gpu: &Gpu, builder: FontBuilder) -> Self {
        let scale = rusttype::Scale::uniform(Self::RES);
        let font = match builder {
            FontBuilder::Ref(bytes) => rusttype::Font::try_from_bytes(bytes).unwrap(),
            FontBuilder::Owned(bytes) => rusttype::Font::try_from_vec(bytes).unwrap(),
        };
        let face_ref = match &font {
            rusttype::Font::Ref(f) => f,
            rusttype::Font::Owned(f) => f.as_face_ref(),
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
        let mut size = Vector2::default();
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
            Vector2::new(size.x as u32, Self::RES as u32),
            Vector2::new(amount as u32, 1),
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
                let ratio = Vector2::new(bb.width(), bb.height())
                    .cast::<f32>()
                    .component_div(&size.cast::<f32>());
                index_map.insert(id, (counter, ratio));
                sprite_sheet.write(
                    gpu,
                    counter,
                    Vector2::new(bb.width() as u32, bb.height() as u32),
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
    pub vertex_offset: Isometry2<f32>,
    pub vertex_rotation_axis: Vector2<f32>,
    pub horizontal_alignment: TextAlignment,
    pub vertical_alignment: TextAlignment,
}

impl Default for TextSection<&str> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: "",
            size: 1.0,
            vertex_offset: Isometry2::new(
                MeshBuilder2D::DEFAULT_OFFSET,
                MeshBuilder2D::DEFAULT_ROTATION,
            ),
            vertex_rotation_axis: Vector2::new(0.0, 0.0),
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
            vertex_offset: Isometry2::new(
                MeshBuilder2D::DEFAULT_OFFSET,
                MeshBuilder2D::DEFAULT_ROTATION,
            ),
            vertex_rotation_axis: Vector2::new(0.0, 0.0),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
        }
    }
}

pub struct Text {
    font: Font,
    mesh: Mesh<Vertex2DText>,
}

impl Text {
    pub fn new<S: AsRef<str>>(gpu: &Gpu, font: &Font, sections: &[TextSection<S>]) -> Self {
        let builder = Self::compute_vertices(font, sections);
        let mesh = gpu.create_mesh(builder);
        return Self {
            font: font.clone(),
            mesh,
        };
    }

    fn compute_vertices<S: AsRef<str>>(
        font: &Font,
        sections: &[TextSection<S>],
    ) -> (Vec<Vertex2DText>, Vec<Index>) {
        let mut vertices: Vec<Vertex2DText> = vec![];
        let mut indices: Vec<Index> = vec![];

        fn compute_modifed_vertices(
            vertices: &mut [Vertex2DText],
            vertex_offset: Isometry2<f32>,
            vertex_rotation_axis: Vector2<f32>,
        ) {
            let angle = vertex_offset.rotation.angle();
            if angle != MeshBuilder2D::DEFAULT_ROTATION {
                for v in vertices.iter_mut() {
                    let delta = v.pos - vertex_rotation_axis;
                    v.pos = vertex_rotation_axis + vertex_offset.rotation * delta;
                }
            }

            if vertex_offset.translation.vector != MeshBuilder2D::DEFAULT_OFFSET {
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
                            let size = Vector2::new(bb.width(), bb.height());
                            let bottom_left = Vector2::new(glyph.position().x, glyph.position().y);
                            let top_right = bottom_left + size;
                            let bottom_right = bottom_left + Vector2::new(size.x, 0.0);
                            let top_left = bottom_left + Vector2::new(0.0, size.y);
                            let offset = Vector2::new(-horizontal, -bb.max.y - vertical - off_y);

                            vertices.extend([
                                Vertex2DText {
                                    pos: top_left + offset,
                                    tex: Vector2::new(0.0, 0.0),
                                    color: section.color,
                                    sprite: *id,
                                },
                                Vertex2DText {
                                    pos: bottom_left + offset,
                                    tex: Vector2::new(0.0, scale.y),
                                    color: section.color,
                                    sprite: *id,
                                },
                                Vertex2DText {
                                    pos: bottom_right + offset,
                                    tex: Vector2::new(scale.x, scale.y),
                                    color: section.color,
                                    sprite: *id,
                                },
                                Vertex2DText {
                                    pos: top_right + offset,
                                    tex: Vector2::new(scale.x, 0.0),
                                    color: section.color,
                                    sprite: *id,
                                },
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
        return (vertices, indices);
    }

    pub fn write<S: AsRef<str>>(&mut self, gpu: &Gpu, sections: &[TextSection<S>]) {
        let builder = Self::compute_vertices(&self.font, sections);
        self.mesh.write(gpu, builder);
    }

    pub fn font(&self) -> &SpriteSheet {
        &self.font.inner.sprite_sheet
    }

    pub fn mesh(&self) -> &Mesh<Vertex2DText> {
        &self.mesh
    }
}
