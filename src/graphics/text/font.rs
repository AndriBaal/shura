use crate::Gpu;

/// Font that can be rendered onto a [sprite](crate::Sprite).
pub use wgpu_glyph::BuiltInLineBreaker as DefaultLineBreaker;
pub use wgpu_glyph::FontId;
pub use wgpu_glyph::Layout as LineBreaker;
pub use wgpu_glyph::Text;
use wgpu_glyph::{ab_glyph, GlyphBrush, GlyphBrushBuilder};

pub struct FontBrush {
    pub brush: GlyphBrush<()>,
}

impl FontBrush {
    pub fn new(gpu: &Gpu, bytes: &'static [u8]) -> FontBrush {
        let font = ab_glyph::FontArc::try_from_slice(bytes).unwrap();
        let brush = GlyphBrushBuilder::using_font(font)
            // .multisample_state(gpu.base.multisample_state)
            .build(&gpu.device, gpu.config.format);
        Self { brush }
    }

    pub fn add_font(&mut self, bytes: &'static [u8]) -> FontId {
        let font = ab_glyph::FontArc::try_from_slice(bytes).unwrap();
        self.brush.add_font(font)
    }
}
