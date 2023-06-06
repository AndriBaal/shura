use crate::AABB;
use crate::text::*;
use crate::Vector;

pub enum Alignment {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Center,
}

/// Section of Text
pub struct TextSection<'a> {
    pub alignment: Alignment,
    pub position: Vector<f32>,
    pub bounds: Vector<f32>,
    pub layout: Layout<BuiltInLineBreaker>,
    pub text: Vec<Text<'a>>
}

impl<'a> TextSection<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn to_glyph_section(
        mut self,
        font: &mut GlyphBrush,
        resolution: f32,
        camera_aabb: AABB
    ) -> Section<'a> {
        for text in &mut self.text {
            text.scale.x *= resolution;
            text.scale.y *= resolution;
        }

        // if self.center_in_camera {
        //     self.position += camera_aabb.dim() / 2.0;
        // }

        self.position += camera_aabb.center();
        self.position *= resolution;
        let mut section = Section {
            screen_position: (self.position.x, self.position.y),
            bounds: (self.bounds.x, self.bounds.y),
            layout: self.layout,
            text: self.text,
        };
        match self.alignment {
            Alignment::TopLeft => {}
            Alignment::TopRight => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.0 -= bounds.width();
                    section.screen_position.1 += bounds.height();
                }
            }
            Alignment::BottomRight => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.0 -= bounds.width();
                    section.screen_position.1 -= bounds.height();
                }
            }
            Alignment::BottomLeft => {
                if let Some(bounds) = font.glyph_bounds(&section) {
                    section.screen_position.1 -= bounds.height();
                }
            }
            Alignment::Center => {
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
            alignment: Alignment::Center,
            position: Vector::new(0.0, 0.0),
            bounds: Vector::new(f32::INFINITY, f32::INFINITY),
            layout: Default::default(),
            text: vec![]
        }
    }
}
