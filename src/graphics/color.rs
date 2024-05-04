#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct RgbaColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RgbaColor {
    pub const TRANSPARENT: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const GOLD: Self = Self {
        r: 219,
        g: 172,
        b: 52,
        a: 255,
    };
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const RED: Self = Self {
        r: 255,
        g: 0,
        b: 0,
        a: 255,
    };
    pub const LIME: Self = Self {
        r: 0,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const BLUE: Self = Self {
        r: 0,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const SILVER: Self = Self {
        r: 191,
        g: 191,
        b: 191,
        a: 255,
    };
    pub const GRAY: Self = Self {
        r: 127,
        g: 127,
        b: 127,
        a: 255,
    };
    pub const DARK_GRAY: Self = Self {
        r: 51,
        g: 51,
        b: 51,
        a: 255,
    };
    pub const BROWN: Self = Self {
        r: 140,
        g: 68,
        b: 17,
        a: 255,
    };
    pub const PURPLE: Self = Self {
        r: 127,
        g: 0,
        b: 127,
        a: 255,
    };
    pub const PINK: Self = Self {
        r: 255,
        g: 0,
        b: 255,
        a: 255,
    };
    pub const GREEN: Self = Self {
        r: 0,
        g: 127,
        b: 0,
        a: 255,
    };
    pub const YELLOW: Self = Self {
        r: 255,
        g: 255,
        b: 0,
        a: 255,
    };
    pub const NAVY: Self = Self {
        r: 0,
        g: 0,
        b: 127,
        a: 255,
    };
    pub const CYAN: Self = Self {
        r: 0,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const ORANGE: Self = Self {
        r: 255,
        g: 165,
        b: 0,
        a: 255,
    };
    pub const DARK_ORANGE: Self = Self {
        r: 255,
        g: 77,
        b: 1,
        a: 255,
    };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn set(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }

    pub fn invert(&mut self) {
        self.r = u8::MAX - self.r;
        self.g = u8::MAX - self.g;
        self.b = u8::MAX - self.b;
    }
}

impl From<RgbaColor> for image::Rgba<u8> {
    fn from(val: RgbaColor) -> Self {
        image::Rgba([val.r, val.g, val.b, val.a])
    }
}

#[cfg(feature = "gui")]
impl From<RgbaColor> for crate::gui::Color32 {
    fn from(val: RgbaColor) -> Self {
        crate::gui::Color32::from_rgba_premultiplied(val.r, val.g, val.b, val.a)
    }
}

impl From<RgbaColor> for [u8; 4] {
    fn from(val: RgbaColor) -> Self {
        [val.r, val.g, val.b, val.a]
    }
}

#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[cfg(feature = "gui")]
impl From<Color> for egui::Rgba {
    fn from(val: Color) -> Self {
        egui::Rgba::from_rgba_premultiplied(val.r, val.g, val.g, val.a)
    }
}

#[cfg(feature = "gui")]
impl From<egui::Rgba> for Color {
    fn from(value: egui::Rgba) -> Self {
        Self {
            r: value.r(),
            g: value.g(),
            b: value.b(),
            a: value.a(),
        }
    }
}

impl Color {
    pub const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const GOLD: Self = Self {
        r: 0.86,
        g: 0.67,
        b: 0.2,
        a: 1.0,
    };
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const RED: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const LIME: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const BLUE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const SILVER: Self = Self {
        r: 0.75,
        g: 0.75,
        b: 0.75,
        a: 1.0,
    };
    pub const GRAY: Self = Self {
        r: 0.5,
        g: 0.5,
        b: 0.5,
        a: 1.0,
    };
    pub const DARK_GRAY: Self = Self {
        r: 0.2,
        g: 0.2,
        b: 0.2,
        a: 1.0,
    };
    pub const BROWN: Self = Self {
        r: 0.55,
        g: 0.27,
        b: 0.07,
        a: 1.0,
    };
    pub const PURPLE: Self = Self {
        r: 0.5,
        g: 0.0,
        b: 0.5,
        a: 1.0,
    };
    pub const PINK: Self = Self {
        r: 1.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    pub const GREEN: Self = Self {
        r: 0.0,
        g: 0.5,
        b: 0.0,
        a: 1.0,
    };
    pub const YELLOW: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 0.0,
        a: 1.0,
    };
    pub const NAVY: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.5,
        a: 1.0,
    };
    pub const CYAN: Self = Self {
        r: 0.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const ORANGE: Self = Self {
        r: 1.0,
        g: 0.65,
        b: 0.0,
        a: 1.0,
    };
    pub const DARK_ORANGE: Self = Self {
        r: 1.0,
        g: 0.3,
        b: 0.01,
        a: 1.0,
    };

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn set(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }

    pub fn invert(&mut self) {
        self.r = 1.0 - self.r;
        self.g = 1.0 - self.g;
        self.b = 1.0 - self.b;
    }
}

impl From<Color> for wgpu::Color {
    fn from(val: Color) -> Self {
        wgpu::Color {
            r: val.r as f64,
            g: val.g as f64,
            b: val.b as f64,
            a: val.a as f64,
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(val: Color) -> Self {
        [val.r, val.g, val.b, val.a]
    }
}

impl From<RgbaColor> for Color {
    fn from(color: RgbaColor) -> Self {
        Color {
            r: color.r as f32 / 255.0,
            g: color.g as f32 / 255.0,
            b: color.b as f32 / 255.0,
            a: color.a as f32 / 255.0,
        }
    }
}

impl From<Color> for RgbaColor {
    fn from(color: Color) -> Self {
        RgbaColor {
            r: (color.r * 255.0) as u8,
            g: (color.g * 255.0) as u8,
            b: (color.b * 255.0) as u8,
            a: (color.a * 255.0) as u8,
        }
    }
}
