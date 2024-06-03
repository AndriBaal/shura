use crate::graphics::Color;
use crate::math::Vector2;
use instant::Duration;

use super::Gpu;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug)]
pub struct ScreenConfig {
    pub clear_color: Option<Color>,
    pub max_fps: Option<u32>,
    #[cfg(feature = "framebuffer")]
    render_scale: f32,
    vsync: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub(crate) changed: bool,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            clear_color: Some(Color::BLACK),
            max_fps: None,
            vsync: false,
            changed: true,
            #[cfg(feature = "framebuffer")]
            render_scale: 1.0,
        }
    }
}

impl ScreenConfig {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }

    pub fn max_fps(&self) -> Option<u32> {
        self.max_fps
    }

    pub fn vsync(&self) -> bool {
        self.vsync
    }

    #[cfg(feature = "framebuffer")]
    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        self.changed = true;
        self.vsync = vsync;
    }

    #[cfg(feature = "framebuffer")]
    pub fn set_render_scale(&mut self, render_scale: f32) {
        self.changed = true;
        self.render_scale = render_scale;
    }

    pub fn set_clear_color(&mut self, clear_color: Option<Color>) {
        self.clear_color = clear_color;
    }

    pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
        self.max_fps = max_fps;
    }

    pub fn max_frame_time(&self) -> Option<Duration> {
        if let Some(max_fps) = self.max_fps {
            return Some(Duration::from_secs_f32(1.0 / max_fps as f32));
        }
        None
    }

    pub fn render_size(&self, gpu: &Gpu) -> Vector2<u32> {
        let surface_size = gpu.surface_size();
        #[cfg(not(feature = "framebuffer"))]
        {
            return surface_size;
        }
        #[cfg(feature = "framebuffer")]
        {
            let size = surface_size.cast::<f32>() * self.render_scale;
            let size = Vector2::new(size.x.max(1.0) as u32, size.y.max(1.0) as u32);
            return size;
        }
    }
}
