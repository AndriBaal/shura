use instant::Duration;

use crate::{Color, Shura};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug)]
pub struct ScreenConfig {
    clear_color: Option<Color>,
    render_scale: f32,
    max_fps: Option<u32>,
}

impl Default for ScreenConfig {
    fn default() -> Self {
        Self {
            clear_color: Some(Color::new(0.0, 0.0, 0.0, 1.0)),
            render_scale: 1.0,
            max_fps: None,
        }
    }
}

impl ScreenConfig {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub fn render_scale(&self) -> f32 {
        self.render_scale
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.clear_color
    }

    pub fn max_fps(&self) -> Option<u32> {
        self.max_fps
    }

    pub fn set_render_scale(&self, shura: &mut Shura, render_scale: f32) {
        shura.defaults.apply_render_scale(&shura.gpu, render_scale);
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
        return None;
    }
}
