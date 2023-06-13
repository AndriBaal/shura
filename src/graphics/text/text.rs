use crate::text::*;
use crate::Vector;

pub enum TextAlignment {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Center,
}

/// Section of Text
pub struct TextSection<'a> {
    pub alignment: TextAlignment,
    pub position: Vector<f32>,
    /// Bounds in halt extents
    pub bounds: Vector<f32>,
    pub layout: Layout<BuiltInLineBreaker>,
    pub text: Vec<Text<'a>>,
}

impl<'a> TextSection<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn to_glyph_section(
        mut self,
        font: &mut GlyphBrush,
        resolution: f32,
        camera_pos: Vector<f32>,
        offset: Vector<f32>,
    ) -> Section<'a> {
        for text in &mut self.text {
            text.scale.x *= resolution;
            text.scale.y *= resolution;
        }

        self.position.y = -self.position.y;
        self.position += offset;

        self.position.x -= camera_pos.x;
        self.position.y += camera_pos.y;
        self.position *= resolution;
        self.bounds *= resolution * 2.0;
        let mut section = Section {
            screen_position: (self.position.x, self.position.y),
            bounds: (self.bounds.x, self.bounds.y),
            layout: self.layout,
            text: self.text,
        };
        match self.alignment {
            TextAlignment::TopLeft => {}
            TextAlignment::TopRight => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.0 -= bounds.width();
                }
            }
            TextAlignment::BottomRight => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.0 -= bounds.width();
                    section.screen_position.1 -= bounds.height();
                }
            }
            TextAlignment::BottomLeft => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.1 -= bounds.height();
                }
            }
            TextAlignment::Center => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.0 -= bounds.width() / 2.0;
                    section.screen_position.1 -= bounds.height() / 2.0;
                }
            }
        }
        return section;
    }
}

impl<'a> Default for TextSection<'a> {
    fn default() -> Self {
        Self {
            alignment: TextAlignment::Center,
            position: Vector::new(0.0, 0.0),
            bounds: Vector::new(f32::INFINITY, f32::INFINITY),
            layout: Default::default(),
            text: vec![],
        }
    }
}
