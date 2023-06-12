use super::{text::TextSection, text_cache::TextCache};
use crate::{
    text::TextVertex, Gpu, GpuDefaults, Matrix, RenderConfig, RenderConfigTarget, RenderEncoder,
    Vector,
};
use glyph_brush::{
    ab_glyph::{FontRef, InvalidFont},
    BrushAction, DefaultSectionHasher, Extra,
};
use std::sync::Mutex;

pub(crate) type GlyphBrush =
    glyph_brush::GlyphBrush<TextVertex, Extra, FontRef<'static>, DefaultSectionHasher>;

pub struct FontBrush {
    inner: Mutex<GlyphBrush>,
    cache: TextCache,
}

impl FontBrush {
    pub fn new(gpu: &Gpu, data: &'static [u8], chars: u64) -> Result<FontBrush, InvalidFont> {
        let font = FontRef::try_from_slice(data).unwrap();
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

    pub fn queue(&self, defaults: &GpuDefaults, config: RenderConfig, sections: Vec<TextSection>) {
        let cam = config.camera.camera(defaults);
        let target = config.target.target(defaults);
        let cam_aabb = cam.model().aabb(Default::default());
        let camera_pos = cam_aabb.center();
        let dim = cam_aabb.dim();
        let resolution = target.size().x as f32 / cam_aabb.dim().x;
        let offset = dim / 2.0;
        let mut inner = self.inner.lock().unwrap();
        for s in sections {
            let section = s.to_glyph_section(&mut inner, resolution, camera_pos, offset);
            inner.queue(section);
        }
    }

    pub fn submit(&self, encoder: &mut RenderEncoder, target: RenderConfigTarget) {
        let target = target.target(encoder.defaults);
        let mut pass = encoder
            .inner
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target.msaa(),
                    resolve_target: Some(target.view()),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],

                depth_stencil_attachment: None,
            });
        self.render(encoder.gpu, &mut pass, target.size().cast::<f32>())
    }

    pub(crate) fn render<'a>(
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
