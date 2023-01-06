use crate::Gpu;

/// Font that can be rendered onto a [sprite](crate::Sprite).
pub type Font = wgpu_glyph::GlyphBrush<()>;
pub use wgpu_glyph::Layout as LineBreaker;
pub use wgpu_glyph::BuiltInLineBreaker as DefaultLineBreaker;
pub use wgpu_glyph::Text as Text;

pub(crate) trait CreateFontWgpu {
    fn new_font_wgpu(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        bytes: &'static [u8],
    )-> Font {
        use wgpu_glyph::{ab_glyph, GlyphBrushBuilder};
        let inconsolata = ab_glyph::FontArc::try_from_slice(bytes).unwrap();

        GlyphBrushBuilder::using_font(inconsolata).build(&device, format)
    }
}

pub trait CreateFont {
    fn new_font(gpu: &Gpu, bytes: &'static [u8]) -> Font {
        use wgpu_glyph::{ab_glyph, GlyphBrushBuilder};
        let inconsolata = ab_glyph::FontArc::try_from_slice(bytes).unwrap();

        GlyphBrushBuilder::using_font(inconsolata).build(&gpu.device, gpu.config.format)
    }
}

impl CreateFontWgpu for Font {}
impl CreateFont for Font {}
