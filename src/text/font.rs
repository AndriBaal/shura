use std::sync::Arc;

use owned_ttf_parser::AsFaceRef;
use rustc_hash::FxHashMap;

use crate::{
    assets::load_asset_bytes_async,
    graphics::{Gpu, SpriteSheet, SpriteSheetBuilder, SpriteSheetIndex},
    math::Vector2,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::assets::load_asset_bytes;

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

    pub async fn asset_async(path: &str) -> Self {
        let bytes = load_asset_bytes_async(path).await.unwrap();
        Self::Owned(bytes)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn asset(path: &str) -> Self {
        let bytes = load_asset_bytes(path).unwrap();
        Self::Owned(bytes)
    }
}

#[derive(Clone)]
pub struct Font {
    pub(super) inner: Arc<FontInner>,
}

impl Font {
    pub fn new(gpu: &Gpu, builder: FontBuilder) -> Self {
        let inner = FontInner::new(gpu, builder);
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn sprite_sheet(&self) -> &SpriteSheet {
        return &self.inner.sprite_sheet
    }
}

pub(super) struct FontInner {
    pub(super) sprite_sheet: SpriteSheet,
    pub(super) index_map: FxHashMap<rusttype::GlyphId, (SpriteSheetIndex, Vector2<f32>)>,
    pub(super) font: rusttype::Font<'static>,
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

        Self {
            sprite_sheet,
            index_map,
            font,
        }
    }
}
