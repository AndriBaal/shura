use super::{text::TextSection, text_cache::TextCache};
use crate::{text::TextVertex, Gpu, GpuDefaults, Matrix, RenderConfig, Vector};
use glyph_brush::{
    ab_glyph::{FontArc, InvalidFont},
    BrushAction, DefaultSectionHasher, Extra,
};
use std::sync::Mutex;

pub(crate) type GlyphBrush =
    glyph_brush::GlyphBrush<TextVertex, Extra, FontArc, DefaultSectionHasher>;

pub struct FontBrush {
    inner: Mutex<GlyphBrush>,
    cache: TextCache,
}

impl FontBrush {
    pub fn new(gpu: &Gpu, data: &'static [u8], chars: u64) -> Result<FontBrush, InvalidFont> {
        let font = FontArc::try_from_slice(data).unwrap();
        let cache_size = gpu.device.limits().max_texture_dimension_2d;
        Ok(Self {
            cache: TextCache::new(gpu, Vector::new(cache_size, cache_size), chars),
            inner: Mutex::new(
                glyph_brush::GlyphBrushBuilder::using_font(font)
                    .initial_cache_size((cache_size, cache_size))
                    .build(),
            ),
        })
    }

    pub fn queue(&self, defaults: &GpuDefaults, sections: Vec<TextSection>, config: RenderConfig) {
        let cam = config.camera.camera(defaults);
        let target = config.target.target(defaults);
        let cam_aabb = cam.model().aabb(Default::default());
        let resolution = target.size().x as f32 / cam_aabb.dim().x;
        let mut inner = self.inner.lock().unwrap();
        for s in sections {
            let section = s.to_glyph_section(&mut inner, resolution, cam_aabb);
            inner.queue(section);
        }
    }

    pub fn render<'a>(
        &'a self,
        gpu: &'a Gpu,
        pass: &mut wgpu::RenderPass<'a>,
        target_size: Vector<f32>,
    ) {
        let mut inner = self.inner.lock().unwrap();
        let pipeline = &gpu.base.text_pipeline;
        // Process sections:
        loop {
            // Contains BrushAction enum which marks for
            // drawing or redrawing (using old data).
            let brush_action = inner.process_queued(
                |rect, data| self.cache.update_texture(gpu, rect, data),
                TextVertex::to_vertex,
            );

            match brush_action {
                Ok(action) => {
                    break match action {
                        BrushAction::Draw(vertices) => {
                            self.cache.update_vertex_buffer(gpu, vertices);
                            self.cache.update_matrix(gpu, Matrix::ortho(target_size));
                            pipeline.draw(&self.cache, pass);
                        }
                        BrushAction::ReDraw => pipeline.draw(&self.cache, pass),
                    }
                }
                Err(glyph_brush::BrushError::TextureTooSmall { suggested }) => {
                    let max_image_dimension = gpu.device.limits().max_texture_dimension_2d;
                    if suggested.0 > max_image_dimension || suggested.1 > max_image_dimension {
                        if inner.texture_dimensions().0 > max_image_dimension
                            || inner.texture_dimensions().1 > max_image_dimension
                        {
                            panic!("Font texture size surpasses limit!");
                        }
                    }
                }
            }
        }
    }

    // #[inline]
    // pub(crate) fn glyph_bounds(&mut self, resolution: f32, section: TextSection) -> Option<AABB> {
    //     if let Some(rect) = self
    //         .inner
    //         .lock()
    //         .unwrap()
    //         .glyph_bounds(section.to_glyph_section(resolution))
    //     {
    //         return Some(AABB::new(
    //             Vector::new(rect.min.x, rect.min.y),
    //             Vector::new(rect.max.x, rect.max.y),
    //         ));
    //     }
    //     return None;
    // }

    // pub fn glyphs_iter<'a, 'b, S>(&'b mut self, section: TextSection) -> SectionGlyphIter<'b>
    // {
    //     self.inner.lock().unwrap().glyphs(section.to_glyph_section(resolution))
    // }

    // pub fn fonts(&self) -> &[FontArc] {
    //     self.inner.lock().unwrap().fonts()
    // }
}
