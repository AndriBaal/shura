use instant::Duration;

use crate::Color;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug)]
/// Configurations of handling the Screen. This can be configured per [Scene](crate::Scene)
pub struct ScreenConfig {
    pub clear_color: Option<Color>,
    pub max_fps: Option<u32>,
    pub vsync: bool,
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
        }
    }
}

impl ScreenConfig {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub fn resized(&self) -> bool {
        self.changed
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

    pub fn set_vsync(&mut self, vsync: bool) {
        self.changed = true;
        self.vsync = vsync;
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
