use super::{text::TextSection, text_cache::TextCache};
use crate::{text::TextVertex, Gpu, GpuDefaults, Matrix, RenderConfig, Vector};
use glyph_brush::{
    ab_glyph::{FontArc, InvalidFont, Rect},
    BrushAction, DefaultSectionHasher, Extra, GlyphCruncher, Section, SectionGlyphIter,
};
use std::{error::Error, fmt::Display};

pub struct FontBrush {
    pub(crate) inner: glyph_brush::GlyphBrush<TextVertex, Extra, FontArc, DefaultSectionHasher>,
    cache: TextCache,
}

impl FontBrush {
    pub fn new(gpu: &Gpu, data: &'static [u8]) -> Result<FontBrush, InvalidFont> {
        let font = FontArc::try_from_slice(data).unwrap();
        let cache_size = gpu.device.limits().max_texture_dimension_2d;
        Ok(Self {
            cache: TextCache::new(gpu, Vector::new(cache_size, cache_size)),
            inner: glyph_brush::GlyphBrushBuilder::using_font(font)
                .initial_cache_size((cache_size, cache_size))
                .build(),
        })
    }

    pub fn queue(
        &mut self,
        defaults: &GpuDefaults,
        sections: Vec<TextSection>,
        config: RenderConfig,
    ) {
        let cam = config.camera.camera(defaults);
        let target = config.target.target(defaults);
        let resolution = target.size().x as f32 / cam.model().aabb(Default::default()).dim().x;
        for s in sections {
            self.inner.queue(s.to_glyph_section(resolution));
        }
    }

    pub fn buffer(&mut self, gpu: &Gpu) -> Result<(), BrushError> {
        // Process sections:
        loop {
            // Contains BrushAction enum which marks for
            // drawing or redrawing (using old data).
            let brush_action = self.inner.process_queued(
                |rect, data| self.cache.update_texture(gpu, rect, data),
                TextVertex::to_vertex,
            );

            match brush_action {
                Ok(action) => {
                    break match action {
                        BrushAction::Draw(vertices) => {
                            self.cache.update_vertex_buffer(gpu, vertices)
                        }
                        BrushAction::ReDraw => (),
                    }
                }

                Err(glyph_brush::BrushError::TextureTooSmall { suggested }) => {
                    if log::log_enabled!(log::Level::Warn) {
                        log::warn!(
                            "Resizing cache texture! This should be avoided \
                            by building TextBrush with BrushBuilder::initial_cache_size() \
                            and providing bigger cache texture dimensions."
                        );
                    }
                    // Texture resizing:
                    let max_image_dimension = gpu.device.limits().max_texture_dimension_2d;
                    let (width, height) =
                        if suggested.0 > max_image_dimension || suggested.1 > max_image_dimension {
                            if self.inner.texture_dimensions().0 < max_image_dimension
                                || self.inner.texture_dimensions().1 < max_image_dimension
                            {
                                (max_image_dimension, max_image_dimension)
                            } else {
                                return Err(BrushError::TooBigCacheTexture(max_image_dimension));
                            }
                        } else {
                            suggested
                        };
                    self.cache.recreate_texture(gpu, Vector::new(width, height));
                    self.inner.resize_texture(width, height);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn render<'a>(
        &'a self,
        gpu: &'a Gpu,
        pass: &mut wgpu::RenderPass<'a>,
        target_size: Vector<f32>,
    ) {
        let pipeline = &gpu.base.text_pipeline;
        self.cache.update_matrix(gpu, Matrix::ortho(target_size));
        pipeline.draw(&self.cache, pass);
    }

    #[inline]
    pub fn glyph_bounds<'a, S>(&mut self, section: S) -> Option<Rect>
    where
        S: Into<std::borrow::Cow<'a, Section<'a>>>,
    {
        self.inner.glyph_bounds(section)
    }

    #[inline]
    pub fn glyphs_iter<'a, 'b, S>(&'b mut self, section: S) -> SectionGlyphIter<'b>
    where
        S: Into<std::borrow::Cow<'a, Section<'a>>>,
    {
        self.inner.glyphs(section)
    }

    pub fn fonts(&self) -> &[FontArc] {
        self.inner.fonts()
    }
}

#[derive(Debug)]
pub enum BrushError {
    TooBigCacheTexture(u32),
}

impl Error for BrushError {}

impl Display for BrushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wgpu-text: ")?;
        match self {
            BrushError::TooBigCacheTexture(dimensions) => write!(
                f,
                "While trying to resize the \
                cache texture, the 'wgpu::Limits {{ max_texture_dimension_2d }}' \
                limit of {} was crossed!\n\
                Resizing the cache texture should be avoided \
                from the start by building TextBrush with \
                BrushBuilder::initial_cache_size() and providing bigger cache \
                texture dimensions.",
                dimensions
            ),
        }
    }
}
