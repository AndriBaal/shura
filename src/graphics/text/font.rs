use crate::Gpu;
use wgpu_text::{BrushBuilder, TextBrush, font::FontRef};
use std::{sync::RwLock, ops::DerefMut};

pub struct Font {
    // brush: RwLock<TextBrush<FontRef<'static>>>
    pub(crate) brush: TextBrush<FontRef<'static>>
}

impl Font {
    pub fn new(gpu: &Gpu, bytes: &'static [u8]) -> Font {
        let brush = BrushBuilder::using_font_bytes(bytes)
            .unwrap()
            .initial_cache_size((512, 512))
            .with_multisample(gpu.base.multisample)
            // .texture_filter_method(wgpu::FilterMode::Linear)
            .build(&gpu.device,  gpu.config.width,  gpu.config.height, gpu.config.format);

        Self {brush}
        // Self { brush: Rmutlisample_statemutlisample_statewLock::new(brush) }
    }

    // pub(crate) fn get(&self) -> impl DerefMut<Target=TextBrush<FontRef<'static>>> + '_ {
    //     self.brush.write().ok().unwrap()
    // }
}
