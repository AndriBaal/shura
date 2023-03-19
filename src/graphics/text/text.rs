use crate::{
    text::{DefaultLineBreaker, FontBrush, LineBreaker, Text},
    Color, Vector,
};

pub struct TextSection<'a> {
    pub position: Vector<f32>,
    pub bounds: Vector<f32>,
    pub layout: LineBreaker<DefaultLineBreaker>,
    pub text: Vec<Text<'a>>,
}

pub struct TextDescriptor<'a> {
    pub clear_color: Option<Color>,
    pub sections: Vec<TextSection<'a>>,
    pub font: &'a mut FontBrush,
}

impl<'a> TextSection<'a> {
    pub fn to_glyph_section(self) -> wgpu_glyph::Section<'a> {
        wgpu_glyph::Section {
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
