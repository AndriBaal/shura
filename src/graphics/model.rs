use crate::{Dimension, Gpu, Index, Isometry, Rotation, Vector, Vertex};
use wgpu::util::DeviceExt;

// #[cfg(feature = "physics")]
// use rapier2d::prelude::ColliderBuilder;

#[derive(Copy, Clone)]
/// Shape of a [Model].
pub enum ModelShape {
    Ball { radius: f32 },
    Cuboid { dim: Dimension<f32> },
    Custom,
}

/// Builder to easily create a [Model].
pub struct ModelBuilder {
    vertices: Vec<Vertex>,
    indices: Vec<Index>,
    position: Isometry<f32>,
    scale: Vector<f32>,
    shape: ModelShape,
}

impl ModelBuilder {
    pub fn cuboid(dim: Dimension<f32>) -> Self {
        Self {
            position: Isometry::default(),
            scale: Vector::new(1.0, 1.0),
            vertices: vec![
                Vertex::new(Vector::new(-dim.width, dim.height), Vector::new(0.0, 0.0)),
                Vertex::new(Vector::new(-dim.width, -dim.height), Vector::new(0.0, 1.0)),
                Vertex::new(Vector::new(dim.width, -dim.height), Vector::new(1.0, 1.0)),
                Vertex::new(Vector::new(dim.width, dim.height), Vector::new(1.0, 0.0)),
            ],
            indices: vec![Index::new(0, 1, 2), Index::new(2, 3, 0)],
            shape: ModelShape::Cuboid { dim: dim * 2.0 },
        }
    }

    // #[cfg(feature = "physics")]
    // pub fn from_collider(collider: ColliderBuilder) {
    // }

    pub fn ball(radius: f32, amount_points: u16) -> Self {
        const PI: f32 = std::f32::consts::PI;
        const MIN_POINTS: u16 = 3;
        assert!(
            amount_points >= MIN_POINTS,
            "A Ball must have at least {} points!",
            MIN_POINTS
        );
        let mut vertices = vec![Vertex::new(Vector::new(0.0, 0.0), Vector::new(0.5, 0.5))];
        let mut indices = vec![];
        for i in 1..amount_points + 1 {
            let i = i as f32;
            let pos = Vector::new(
                radius * (i / amount_points as f32 * 2.0 * PI).cos(),
                radius * (i / amount_points as f32 * 2.0 * PI).sin(),
            );

            vertices.push(Vertex {
                pos,
                tex_coords: Vector::new(
                    (i / amount_points as f32 * 2.0 * PI).cos() / 2.0 + 0.5,
                    (i / amount_points as f32 * 2.0 * PI).sin() / -2.0 + 0.5,
                ),
            });
        }

        for i in 0..amount_points {
            indices.push(Index::new(0, i, i + 1));
        }
        indices.push(Index::new(0, amount_points, 1));

        Self {
            position: Isometry::default(),
            scale: Vector::new(1.0, 1.0),
            vertices,
            indices,
            shape: ModelShape::Ball { radius },
        }
    }

    pub fn custom(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self {
            position: Isometry::default(),
            scale: Vector::new(1.0, 1.0),
            vertices,
            indices,
            shape: ModelShape::Custom,
        }
    }

    pub fn scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn position(mut self, position: Isometry<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn rotation(mut self, rotation: Rotation<f32>) -> Self {
        self.position.rotation = rotation;
        self
    }

    pub fn translation(mut self, translation: Vector<f32>) -> Self {
        self.position.translation.vector = translation;
        self
    }

    pub fn build(self, gpu: &Gpu) -> Model {
        self.build_wgpu(&gpu.device)
    }

    pub(crate) fn build_wgpu(mut self, device: &wgpu::Device) -> Model {
        fn rotate_point_around_origin(
            origin: Vector<f32>,
            point: Vector<f32>,
            rot: Rotation<f32>,
        ) -> Vector<f32> {
            let sin = rot.sin_angle();
            let cos = rot.cos_angle();
            return Vector::new(
                origin.x + (point.x - origin.x) * cos - (point.y - origin.y) * sin,
                origin.y + (point.x - origin.x) * sin + (point.y - origin.y) * cos,
            );
        }

        for v in &mut self.vertices {
            v.pos.x *= self.scale.x;
            v.pos.y *= self.scale.y;
        }

        let angle = self.position.rotation.angle();
        if angle != 0.0 {
            for v in &mut self.vertices {
                v.pos = rotate_point_around_origin(
                    Vector::new(0.0, 0.0),
                    v.pos,
                    self.position.rotation,
                );
            }
        }

        for v in &mut self.vertices {
            v.pos += self.position.translation.vector;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&self.vertices[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&self.indices[..]),
            usage: wgpu::BufferUsages::INDEX,
        });

        Model {
            amount_of_vertices: self.vertices.len() as u32,
            amount_of_indices: self.indices.len() as u32,
            vertex_buffer,
            index_buffer,
            shape: self.shape,
        }
    }
}

/// 2D Model represented by its [Vertices](Vertex) and [Indices](Index).
pub struct Model {
    amount_of_vertices: u32,
    amount_of_indices: u32,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    shape: ModelShape,
}

impl Model {
    pub fn new(gpu: &Gpu, builder: ModelBuilder) -> Self {
        builder.build(gpu)
    }

    pub(crate) fn new_wgpu(device: &wgpu::Device, builder: ModelBuilder) -> Self {
        builder.build_wgpu(device)
    }

    pub fn write(&mut self, gpu: &Gpu, vertices: &[Vertex], indices: &[Index]) {
        self.write_indices(gpu, indices);
        self.write_vertices(gpu, vertices);
    }

    pub fn write_vertices(&mut self, gpu: &Gpu, vertices: &[Vertex]) {
        assert_eq!(vertices.len(), self.amount_of_vertices as usize);
        gpu.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices[..]));
    }

    pub fn write_indices(&mut self, gpu: &Gpu, indices: &[Index]) {
        assert_eq!(indices.len(), self.amount_of_indices as usize);
        gpu.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&indices[..]));
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &wgpu::Buffer {
        &self.index_buffer
    }

    pub fn amount_of_indices(&self) -> u32 {
        self.amount_of_indices * 3
    }

    pub fn amount_of_vertices(&self) -> u32 {
        self.amount_of_vertices
    }

    pub fn shape(&self) -> ModelShape {
        self.shape
    }
}
