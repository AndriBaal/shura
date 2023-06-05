use crate::Vector;
use crate::text::*;

/// Section of Text
pub struct TextSection<'a> {
    pub position: Vector<f32>,
    pub bounds: Vector<f32>,
    pub layout: Layout<BuiltInLineBreaker>,
    pub text: Vec<Text<'a>>,
}

impl<'a> TextSection<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_glyph_section(mut self, resolution: f32) -> Section<'a> {
        for text in &mut self.text {
            text.scale.x *= resolution;
            text.scale.y *= resolution;
        }
        self.position.x *= resolution;
        self.position.y *= resolution;
        Section {
            screen_position: (self.position.x, self.position.y),
            bounds: (self.bounds.x, self.bounds.y),
            layout: self.layout,
            text: self.text,
        }
    }
}

impl<'a> Default for TextSection<'a> {
    fn default() -> Self {
        Self {
            position: Vector::new(0.0, 0.0),
            bounds: Vector::new(f32::INFINITY, f32::INFINITY),
            layout: Default::default(),
            text: vec![],
        }
    }
}
