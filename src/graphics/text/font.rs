use crate::{text::TextSection, Gpu, GpuDefaults, RenderConfig, Vector};
use std::{ops::DerefMut, sync::RwLock};
use wgpu::RenderPass;
use wgpu_text::{font::FontRef, BrushBuilder, TextBrush};

pub struct Font {
    // brush: RwLock<TextBrush<FontRef<'static>>>
    pub(crate) brush: TextBrush<FontRef<'static>>,
}

impl Font {
    pub fn new(gpu: &Gpu, bytes: &'static [u8]) -> Font {
        let brush = BrushBuilder::using_font_bytes(bytes)
            .unwrap()
            .initial_cache_size((512, 512))
            .with_multisample(gpu.base.multisample)
            // .texture_filter_method(wgpu::FilterMode::Linear)
            .build(
                &gpu.device,
                gpu.config.width,
                gpu.config.height,
                gpu.config.format,
            );

        Self { brush }
        // Self { brush: Rmutlisample_statemutlisample_statewLock::new(brush) }
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
        let sections = sections
            .into_iter()
            .map(|s| s.to_glyph_section(resolution))
            .collect();
        self.brush.queue(sections);
    }

    pub fn buffer(&mut self, gpu: &Gpu) {
        self.brush.process(&gpu.device, &gpu.queue).unwrap()
    }

    pub(crate) fn render<'a>(
        &'a self,
        gpu: &Gpu,
        pass: &mut RenderPass<'a>,
        target_size: Vector<f32>,
    ) {
        self.brush.resize_view(target_size.x, target_size.y, &gpu.queue);
        self.brush.draw(pass);
    }

    // pub(crate) fn get(&self) -> impl DerefMut<Target=TextBrush<FontRef<'static>>> + '_ {
    //     self.brush.write().ok().unwrap()
    // }
}
