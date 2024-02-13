use crate::{
    component::Component,
    entity::EntityHandle,
    graphics::{
        Color, Gpu, Index, Instance, Instance2D, Mesh, MeshBuilder2D, RenderGroup, SpriteAtlas,
        SpriteSheetIndex, Vertex,
    },
    math::{Isometry2, Matrix2, Vector2},
    physics::World,
    text::Font,
};
use wgpu::vertex_attr_array;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex2DText {
    pub pos: Vector2<f32>,
    pub tex: Vector2<f32>,
    pub color: Color,
    pub sprite: u32,
}

impl Vertex for Vertex2DText {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Uint32,
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
    pub horizontal_alignment: TextAlignment,
    pub vertical_alignment: TextAlignment,
}

impl Default for TextSection<&str> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: "",
            size: 1.0,
            offset: Isometry2::new(
                MeshBuilder2D::DEFAULT_OFFSET,
                MeshBuilder2D::DEFAULT_ROTATION,
            ),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
        }
    }
}

impl Default for TextSection<String> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: String::from(""),
            size: 1.0,
            offset: Isometry2::new(
                MeshBuilder2D::DEFAULT_OFFSET,
                MeshBuilder2D::DEFAULT_ROTATION,
            ),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
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

            let scale = rusttype::Scale::uniform(section.size);
            let metrics = font.inner.font.v_metrics(scale);
            let mut off_y = 0.0;
            for line in text.lines() {
                let glyphs = font
                    .inner
                    .font
                    .layout(line, scale, rusttype::Point::default())
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
                        if let Some((id, scale)) = font.inner.index_map.get(&glyph.id()) {
                            let size = Vector2::new(bb.width(), bb.height());
                            let offset = Vector2::new(-horizontal, -bb.max.y - vertical - off_y);
                            let bottom_left =
                                Vector2::new(glyph.position().x, glyph.position().y) + offset;
                            letter(FormattedGlyph {
                                size,
                                bottom_left,
                                section,
                                scale: *scale,
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
    font: Font,
    mesh: Mesh<Vertex2DText>,
}

impl TextMesh {
    pub fn new<S: AsRef<str>>(gpu: &Gpu, font: &Font, sections: &[TextSection<S>]) -> Self {
        let builder = Self::compute_vertices(font, sections);
        let mesh = gpu.create_mesh(&builder);
        Self {
            font: font.clone(),
            mesh,
        }
    }

    fn compute_vertices<S: AsRef<str>>(
        font: &Font,
        sections: &[TextSection<S>],
    ) -> (Vec<Vertex2DText>, Vec<Index>) {
        let mut vertices: Vec<Vertex2DText> = vec![];
        let mut indices: Vec<Index> = vec![];

        fn compute_modifed_vertices(vertices: &mut [Vertex2DText], offset: Isometry2<f32>) {
            let angle = offset.rotation.angle();
            if angle != MeshBuilder2D::DEFAULT_ROTATION {
                for v in vertices.iter_mut() {
                    // let delta = v.pos - vertex_rotation_axis;
                    // v.pos = vertex_rotation_axis + offset.rotation * delta;
                    v.pos = offset.rotation * v.pos;
                }
            }

            if offset.translation.vector != MeshBuilder2D::DEFAULT_OFFSET {
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
                Vertex2DText {
                    pos: top_left,
                    tex: Vector2::new(0.0, 0.0),
                    color: letter.section.color,
                    sprite: letter.id,
                },
                Vertex2DText {
                    pos: letter.bottom_left,
                    tex: Vector2::new(0.0, letter.scale.y),
                    color: letter.section.color,
                    sprite: letter.id,
                },
                Vertex2DText {
                    pos: bottom_right,
                    tex: Vector2::new(letter.scale.x, letter.scale.y),
                    color: letter.section.color,
                    sprite: letter.id,
                },
                Vertex2DText {
                    pos: top_right,
                    tex: Vector2::new(letter.scale.x, 0.0),
                    color: letter.section.color,
                    sprite: letter.id,
                },
            ]);
            let offset = vertices.len() - 4;
            compute_modifed_vertices(&mut vertices[offset..], letter.section.offset);
            indices.extend([
                Index::new(base_index, base_index + 1, base_index + 2),
                Index::new(base_index + 2, base_index + 3, base_index),
            ]);
        });

        (vertices, indices)
    }

    pub fn write<S: AsRef<str>>(&mut self, gpu: &Gpu, sections: &[TextSection<S>]) {
        let builder = Self::compute_vertices(&self.font, sections);
        self.mesh.write(gpu, builder);
    }

    pub fn font(&self) -> &Font {
        &self.font
    }

    pub fn mesh(&self) -> &Mesh<Vertex2DText> {
        &self.mesh
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LetterInstance2D(pub Instance2D);
impl Instance for LetterInstance2D {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = Instance2D::ATTRIBUTES;
}

pub struct Letter {
    pub pos: Isometry2<f32>,
    pub active: bool,
    pub color: Color,
    pub scale: Vector2<f32>,
    pub atlas: SpriteAtlas,
    pub index: SpriteSheetIndex,
}
pub struct TextComponent2D {
    pub letters: Vec<Letter>,
    pub font: Font,
    pub offset: Isometry2<f32>,
}

impl Component for TextComponent2D {
    type Instance = LetterInstance2D;
    fn buffer(&self, _world: &World, render_group: &mut RenderGroup<Self::Instance>) {
        for letter in &self.letters {
            let rotation = letter.pos.rotation * self.offset.rotation;
            let instance = LetterInstance2D(Instance2D {
                pos: letter.pos.translation.vector + self.offset.translation.vector,
                color: letter.color,
                rot: Matrix2::new(
                    letter.scale.x * rotation.cos_angle(),
                    letter.scale.x * rotation.sin_angle(),
                    letter.scale.y * -rotation.sin_angle(),
                    letter.scale.y * rotation.cos_angle(),
                ),
                atlas: SpriteAtlas::new(letter.scale, Vector2::default()),
                sprite_sheet_index: letter.index,
            });
            render_group.push(instance)
        }
    }
    fn init(&mut self, _handle: EntityHandle, _world: &mut World) {}
    fn finish(&mut self, _world: &mut World) {}
}

impl TextComponent2D {
    pub fn new<S: AsRef<str>>(font: &Font, sections: &[TextSection<S>]) -> Self {
        let letters = Self::compute_instances(font, sections);
        Self {
            letters,
            font: font.clone(),
            offset: Isometry2::default(),
        }
    }

    pub fn write<S: AsRef<str>>(&mut self, sections: &[TextSection<S>]) {
        self.letters = Self::compute_instances(&self.font, sections);
    }

    pub fn compute_instances<S: AsRef<str>>(
        font: &Font,
        sections: &[TextSection<S>],
    ) -> Vec<Letter> {
        let mut instances = vec![];
        TextSection::compute_layout(font, sections, |letter| {
            let rotation = letter.section.offset.rotation;
            let rotation_axis = letter.section.offset.translation.vector;
            let mut pos = letter.bottom_left + letter.size / 2.0;

            let delta = pos - rotation_axis;
            pos = rotation_axis + rotation * delta;

            instances.push(Letter {
                pos: Isometry2::new(pos, rotation.angle()),
                active: true,
                color: letter.section.color,
                scale: letter.size,
                atlas: SpriteAtlas::new(letter.scale, Vector2::default()),
                index: letter.id,
            });
        });
        return instances;
    }
}

struct FormattedGlyph<'a, S: AsRef<str>> {
    size: Vector2<f32>,
    scale: Vector2<f32>,
    bottom_left: Vector2<f32>,
    section: &'a TextSection<S>,
    id: u32,
}
