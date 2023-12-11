use crate::{
    graphics::{
        Color, Gpu, Index, Mesh, MeshBuilder2D, SpriteSheet,
        Vertex,
    },
    text::Font,
    math::{Vector2, Isometry2},
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
    pub vertex_offset: Isometry2<f32>,
    pub vertex_rotation_axis: Vector2<f32>,
    pub horizontal_alignment: TextAlignment,
    pub vertical_alignment: TextAlignment,
}

impl Default for TextSection<&str> {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            text: "",
            size: 1.0,
            vertex_offset: Isometry2::new(
                MeshBuilder2D::DEFAULT_OFFSET,
                MeshBuilder2D::DEFAULT_ROTATION,
            ),
            vertex_rotation_axis: Vector2::new(0.0, 0.0),
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
            vertex_offset: Isometry2::new(
                MeshBuilder2D::DEFAULT_OFFSET,
                MeshBuilder2D::DEFAULT_ROTATION,
            ),
            vertex_rotation_axis: Vector2::new(0.0, 0.0),
            horizontal_alignment: TextAlignment::Start,
            vertical_alignment: TextAlignment::Start,
        }
    }
}

pub struct Text {
    font: Font,
    mesh: Mesh<Vertex2DText>,
}

impl Text {
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

        fn compute_modifed_vertices(
            vertices: &mut [Vertex2DText],
            vertex_offset: Isometry2<f32>,
            vertex_rotation_axis: Vector2<f32>,
        ) {
            let angle = vertex_offset.rotation.angle();
            if angle != MeshBuilder2D::DEFAULT_ROTATION {
                for v in vertices.iter_mut() {
                    let delta = v.pos - vertex_rotation_axis;
                    v.pos = vertex_rotation_axis + vertex_offset.rotation * delta;
                }
            }

            if vertex_offset.translation.vector != MeshBuilder2D::DEFAULT_OFFSET {
                for v in vertices.iter_mut() {
                    v.pos += vertex_offset.translation.vector;
                }
            }
        }
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
                        let base_index = vertices.len() as u32;
                        if let Some((id, scale)) = font.inner.index_map.get(&glyph.id()) {
                            let size = Vector2::new(bb.width(), bb.height());
                            let bottom_left = Vector2::new(glyph.position().x, glyph.position().y);
                            let top_right = bottom_left + size;
                            let bottom_right = bottom_left + Vector2::new(size.x, 0.0);
                            let top_left = bottom_left + Vector2::new(0.0, size.y);
                            let offset = Vector2::new(-horizontal, -bb.max.y - vertical - off_y);

                            vertices.extend([
                                Vertex2DText {
                                    pos: top_left + offset,
                                    tex: Vector2::new(0.0, 0.0),
                                    color: section.color,
                                    sprite: *id,
                                },
                                Vertex2DText {
                                    pos: bottom_left + offset,
                                    tex: Vector2::new(0.0, scale.y),
                                    color: section.color,
                                    sprite: *id,
                                },
                                Vertex2DText {
                                    pos: bottom_right + offset,
                                    tex: Vector2::new(scale.x, scale.y),
                                    color: section.color,
                                    sprite: *id,
                                },
                                Vertex2DText {
                                    pos: top_right + offset,
                                    tex: Vector2::new(scale.x, 0.0),
                                    color: section.color,
                                    sprite: *id,
                                },
                            ]);
                            let offset = vertices.len() - 4;
                            compute_modifed_vertices(
                                &mut vertices[offset..],
                                section.vertex_offset,
                                section.vertex_rotation_axis,
                            );
                            indices.extend([
                                Index::new(base_index, base_index + 1, base_index + 2),
                                Index::new(base_index + 2, base_index + 3, base_index),
                            ]);
                        }
                    }
                }

                off_y += section.size + metrics.descent.abs() + metrics.line_gap;
            }
        }
        (vertices, indices)
    }

    pub fn write<S: AsRef<str>>(&mut self, gpu: &Gpu, sections: &[TextSection<S>]) {
        let builder = Self::compute_vertices(&self.font, sections);
        self.mesh.write(gpu, builder);
    }

    pub fn font(&self) -> &SpriteSheet {
        &self.font.inner.sprite_sheet
    }

    pub fn mesh(&self) -> &Mesh<Vertex2DText> {
        &self.mesh
    }
}
