use owned_ttf_parser::AsFaceRef;
use rustc_hash::FxHashMap;

use crate::{
    graphics::{Gpu, SpriteArray, SpriteArrayBuilder, SpriteArrayIndex},
    math::Vector2,
};

pub enum FontBuilder {
    Ref(&'static [u8]),
    Owned(Vec<u8>),
}

impl FontBuilder {
    pub fn bytes(bytes: &'static [u8]) -> Self {
        Self::Ref(bytes)
    }

    pub fn owned(bytes: Vec<u8>) -> Self {
        Self::Owned(bytes)
    }

    pub fn resource(path: &str) -> Self {
        let resources = crate::app::global_resources();
        let bytes = resources.load_bytes(path).unwrap();
        Self::Owned(bytes)
    }
}

pub struct Font {
    pub(super) sprite_array: SpriteArray,
    pub(super) index_map: FxHashMap<rusttype::GlyphId, (SpriteArrayIndex, Vector2<f32>)>,
    pub(super) font: rusttype::Font<'static>,
}
impl Font {
    const RES: f32 = 400.0;

    pub fn sprite_array(&self) -> &SpriteArray {
        &self.sprite_array
    }

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

        let desc = SpriteArrayBuilder::empty(
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

        let mut sprite_array = gpu.create_sprite_array(desc);
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
                sprite_array.write(
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
            sprite_array,
            index_map,
            font,
        }
    }
}
