use crate::{
    graphics::{Color, Gpu, Index, Mesh, Vertex},
    math::{Isometry2, Vector2},
    text::Font,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TextVertex2D {
    pub pos: Vector2<f32>,
    pub tex: Vector2<f32>,
    pub color: Color,
    pub sprite: u32,
}

impl Vertex for TextVertex2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x4,
        wgpu::VertexFormat::Uint32,
    ];
}

pub enum TextAlignment {
    Start,
    Center,
    End,
}

pub struct TextSection<S: AsRef<str>> {
    pub color: Color,
    pub text: S,
    pub size: f32,
    pub offset: Isometry2<f32>,
    pub rotation_axis: Vector2<f32>,
    pub horizontal_alignment: TextAlignment,
    pub vertical_alignment: TextAlignment,
}

impl Default for TextSection<&str> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: "",
            size: 1.0,
            offset: Isometry2::default(),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
            rotation_axis: Default::default(),
        }
    }
}

impl Default for TextSection<String> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: String::from(""),
            size: 1.0,
            offset: Isometry2::default(),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
            rotation_axis: Default::default(),
        }
    }
}

impl<S: AsRef<str>> TextSection<S> {
    fn compute_layout(
        font: &Font,
        sections: &[TextSection<S>],
        mut letter: impl FnMut(FormattedGlyph<S>),
    ) {
        for section in sections {
            let text = section.text.as_ref();
            if text.is_empty() {
                continue;
            }

            let scaling = rusttype::Scale::uniform(section.size);
            let metrics = font.font.v_metrics(scaling);
            let mut off_y = 0.0;
            for line in text.lines() {
                let glyphs = font
                    .font
                    .layout(line, scaling, rusttype::Point::default())
                    .collect::<Vec<rusttype::PositionedGlyph>>();

                let horizontal = match section.horizontal_alignment {
                    TextAlignment::Start => 0.0,
                    TextAlignment::Center => {
                        let mut max = 0.0;
                        for glyph in glyphs.iter().rev() {
                            if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                                max = glyph.position().x + bb.max.x;
                                break;
                            }
                        }
                        max / 2.0
                    }
                    TextAlignment::End => {
                        let mut max = 0.0;
                        for glyph in glyphs.iter().rev() {
                            if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                                max = glyph.position().x + bb.max.x;
                                break;
                            }
                        }
                        max
                    }
                };
                let vertical = match section.vertical_alignment {
                    TextAlignment::Start => 0.0,
                    TextAlignment::Center => section.size / 2.0,
                    TextAlignment::End => section.size,
                };

                for glyph in &glyphs {
                    if let Some(bb) = glyph.unpositioned().exact_bounding_box() {
                        if let Some((id, scaling)) = font.index_map.get(&glyph.id()) {
                            let size = Vector2::new(bb.width(), bb.height());
                            let offset = Vector2::new(-horizontal, -bb.max.y - vertical - off_y);
                            let bottom_left =
                                Vector2::new(glyph.position().x, glyph.position().y) + offset;
                            letter(FormattedGlyph {
                                size,
                                bottom_left,
                                section,
                                tex_scaling: *scaling,
                                id: *id,
                            })
                        }
                    }
                }

                off_y += section.size + metrics.descent.abs() + metrics.line_gap;
            }
        }
    }
}

pub struct TextMesh {
    mesh: Mesh<TextVertex2D>,
}

impl TextMesh {
    pub fn new<S: AsRef<str>>(gpu: &Gpu, font: &Font, sections: &[TextSection<S>]) -> Self {
        let builder = Self::compute_vertices(font, sections);
        let mesh = gpu.create_mesh(&builder);
        Self { mesh }
    }

    pub fn compute_vertices<S: AsRef<str>>(
        font: &Font,
        sections: &[TextSection<S>],
    ) -> (Vec<TextVertex2D>, Vec<Index>) {
        let mut vertices: Vec<TextVertex2D> = vec![];
        let mut indices: Vec<Index> = vec![];

        fn compute_modifed_vertices(
            vertices: &mut [TextVertex2D],
            offset: Isometry2<f32>,
            rotation_axis: Vector2<f32>,
        ) {
            let angle = offset.rotation.angle();
            if angle != 0.0 {
                for v in vertices.iter_mut() {
                    let delta = v.pos - rotation_axis;
                    v.pos = rotation_axis + offset.rotation * delta;
                    v.pos = offset.rotation * v.pos;
                }
            }

            if offset.translation.vector != Vector2::zeros() {
                for v in vertices.iter_mut() {
                    v.pos += offset.translation.vector;
                }
            }
        }
        TextSection::compute_layout(font, sections, |letter| {
            let base_index = vertices.len() as u32;
            let top_right = letter.bottom_left + letter.size;
            let bottom_right = letter.bottom_left + Vector2::new(letter.size.x, 0.0);
            let top_left = letter.bottom_left + Vector2::new(0.0, letter.size.y);

            vertices.extend([
                TextVertex2D {
                    pos: top_left,
                    tex: Vector2::new(0.0, 0.0),
                    color: letter.section.color,
                    sprite: letter.id,
                },
                TextVertex2D {
                    pos: letter.bottom_left,
                    tex: Vector2::new(0.0, letter.tex_scaling.y),
                    color: letter.section.color,
                    sprite: letter.id,
                },
                TextVertex2D {
                    pos: bottom_right,
                    tex: Vector2::new(letter.tex_scaling.x, letter.tex_scaling.y),
                    color: letter.section.color,
                    sprite: letter.id,
                },
                TextVertex2D {
                    pos: top_right,
                    tex: Vector2::new(letter.tex_scaling.x, 0.0),
                    color: letter.section.color,
                    sprite: letter.id,
                },
            ]);
            let offset = vertices.len() - 4;
            compute_modifed_vertices(
                &mut vertices[offset..],
                letter.section.offset,
                letter.section.rotation_axis,
            );
            indices.extend([
                base_index,
                base_index + 1,
                base_index + 2,
                base_index + 2,
                base_index + 3,
                base_index,
            ]);
        });

        (vertices, indices)
    }

    pub fn write<S: AsRef<str>>(&mut self, gpu: &Gpu, font: &Font, sections: &[TextSection<S>]) {
        let builder = Self::compute_vertices(font, sections);
        self.mesh.write(gpu, builder);
    }

    pub fn mesh(&self) -> &Mesh<TextVertex2D> {
        &self.mesh
    }
}

// pub type LetterInstance2D = Instance2D<LetterData>;
// impl Instance for LetterInstance2D {
//     const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
//         wgpu::VertexFormat::Float32x2,
//         wgpu::VertexFormat::Float32x4,
//         wgpu::VertexFormat::Float32x4,
//         wgpu::VertexFormat::Float32x2,
//         wgpu::VertexFormat::Uint32,
//     ];
// }

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// pub struct LetterData {
//     pub color: Color,
//     pub tex_scaling: Vector2<f32>,
//     pub index: SpriteArrayIndex,
// }

// pub struct Letter {
//     pub color: Color,
//     pub position: Isometry2<f32>,
//     pub scaling: Vector2<f32>,
//     pub tex_scaling: Vector2<f32>,
//     pub index: SpriteArrayIndex,
// }

// pub fn compute_instances<S: AsRef<str>>(
//     font: &Font,
//     sections: &[TextSection<S>],
// ) -> Vec<Letter> {
//     let mut instances = vec![];
//     TextSection::compute_layout(font, sections, |letter: FormattedGlyph<S>| {
//         let mut pos = letter.bottom_left + letter.size / 2.0;

//         let delta = pos - rotation_axis;
//         pos = rotation_axis + rotation * delta;
//         pos = rotation * pos;
//         pos += letter.section.offset.translation.vector;

//         // Letter {
//         //     pos: Isometry2::new(pos, rotation.angle()),
//         //     active: true,
//         //     color: letter.section.color,
//         //     size: letter.size,
//         //     atlas: SpriteAtlas::new(letter.tex_scaling, Vector2::default(), 1.0),
//         //     index: letter.id,
//         // }
//         instances.push(
//             Instance2D::new(position, &(letter.size * scaling), LetterData {
//                 color: letter.section.color,
//                 tex_scaling: letter.tex_scaling,
//                 index: letter.id,
//             }));
//     });
//     instances
// }

struct FormattedGlyph<'a, S: AsRef<str>> {
    size: Vector2<f32>,
    tex_scaling: Vector2<f32>,
    bottom_left: Vector2<f32>,
    section: &'a TextSection<S>,
    id: u32,
}
