#[repr(C)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
/// RGBA color represented by 4 floats between 0 and 1.
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
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
        r: 0.831372549,
        g: 0.68627451,
        b: 0.215686275,
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

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        return Self { r, g, b, a };
    }

    pub fn new_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        return Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        };
    }

    pub fn set(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }

    pub fn set_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.r = r as f32 / 255.0;
        self.g = g as f32 / 255.0;
        self.b = b as f32 / 255.0;
        self.a = a as f32 / 255.0;
    }

    pub fn invert(&mut self) {
        self.r = 1.0 - self.r;
        self.g = 1.0 - self.g;
        self.b = 1.0 - self.b;
    }
}

impl Into<wgpu::Color> for Color {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }
}

impl Into<[f32; 4]> for Color {
    fn into(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Into<image::Rgba<u8>> for Color {
    fn into(self) -> image::Rgba<u8> {
        image::Rgba([
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ])
    }
}
