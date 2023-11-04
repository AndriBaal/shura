#[cfg(feature = "physics")]
use crate::physics::{Shape, TypedShape};
use crate::{
    load_bytes, load_string, Gpu, Isometry2, Matrix2, Rotation2, Sprite, SpriteBuilder, Vector2,
    Vector3, AABB,
};
use std::io::{BufReader, Cursor};
use std::mem;
use std::{
    f32::consts::{FRAC_PI_2, PI},
    marker::PhantomData,
};
use wgpu::util::DeviceExt;
use wgpu::vertex_attr_array;

pub type Mesh2D = Mesh<Vertex2D>;
pub type Mesh3D = Mesh<Vertex3D>;

pub trait MeshBuilder {
    type Vertex;
    fn indices<'a>(&'a self) -> &'a [Index];
    fn vertices<'a>(&'a self) -> &'a [Self::Vertex];
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute];
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    const DESC: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::SIZE,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBUTES,
    };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Represents the order in which [Vertices](Vertex) are drawn in a triangle.
pub struct Index {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Index {
    pub const fn new(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }

    pub fn from_vec(vec: Vec<u32>) -> Vec<Self> {
        assert_eq!(vec.len() % 3, 0);
        let mut indices = Vec::with_capacity(vec.len() / 3);
        for index in vec.chunks(3) {
            indices.push(Index {
                a: index[0],
                b: index[1],
                c: index[2],
            })
        }
        return indices;
    }
}

impl<V: Vertex> MeshBuilder for (Vec<V>, Vec<Index>) {
    type Vertex = V;

    fn indices<'a>(&'a self) -> &'a [Index] {
        &self.1
    }
    fn vertices<'a>(&'a self) -> &'a [Self::Vertex] {
        &self.0
    }
}

/// Single vertex of a mesh. Which hold the coordniate of the vertex and the texture coordinates.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex2D {
    pub pos: Vector2<f32>,
    pub tex: Vector2<f32>,
}

impl Vertex2D {
    pub const fn new(pos: Vector2<f32>, tex: Vector2<f32>) -> Self {
        Vertex2D { pos, tex }
    }
}

impl Vertex for Vertex2D {
    const SIZE: u64 = mem::size_of::<Self>() as u64;
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &[
        wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
            offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
            shader_location: 1,
            format: wgpu::VertexFormat::Float32x2,
        },
    ];
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Builder to easily create a [Mesh].
pub struct MeshBuilder2D {
    pub vertices: Vec<Vertex2D>,
    pub indices: Vec<Index>,
    pub vertex_offset: Isometry2<f32>,
    pub tex_coord_offset: Isometry2<f32>,
    pub vertex_scale: Vector2<f32>,
    pub tex_coord_scale: Vector2<f32>,
    pub vertex_rotation_axis: Vector2<f32>,
    pub tex_coord_rotation_axis: Vector2<f32>,
}

impl MeshBuilder for MeshBuilder2D {
    type Vertex = Vertex2D;

    fn indices<'a>(&'a self) -> &'a [Index] {
        &self.indices
    }
    fn vertices<'a>(&'a self) -> &'a [Self::Vertex] {
        &self.vertices
    }
}

impl MeshBuilder2D {
    pub const TRIANGLE_INDICES: [Index; 1] = [Index::new(0, 1, 2)];
    pub const CUBOID_INDICES: [Index; 2] = [Index::new(0, 1, 2), Index::new(2, 3, 0)];

    pub const DEFAULT_OFFSET: Vector2<f32> = Vector2::new(0.0, 0.0);
    pub const DEFAULT_ROTATION: f32 = 0.0;
    pub const DEFAULT_SCALE: Vector2<f32> = Vector2::new(1.0, 1.0);
    pub fn ball(radius: f32, resolution: u32) -> Self {
        Self::regular_polygon(radius, resolution)
    }

    pub fn capsule(radius: f32, half_height: f32, resolution: u32) -> Self {
        Self::rounded(
            Self::cuboid(Vector2::new(radius, half_height)),
            RoundingDirection::Outward,
            radius,
            resolution,
        )
    }

    pub fn regular_polygon(radius: f32, corners: u32) -> Self {
        const MIN_POINTS: u32 = 3;
        assert!(
            corners >= MIN_POINTS,
            "A Regular Polygon must have at least {} points!",
            MIN_POINTS
        );
        let mut vertices = vec![];
        for i in 0..corners {
            let i = i as f32;

            let pos = Vector2::new(
                radius * (i / corners as f32 * 2.0 * PI).cos(),
                radius * (i / corners as f32 * 2.0 * PI).sin(),
            );

            vertices.push(Vertex2D {
                pos,
                tex: Vector2::new(
                    (i / corners as f32 * 2.0 * PI).cos() / 2.0 + 0.5,
                    (i / corners as f32 * 2.0 * PI).sin() / -2.0 + 0.5,
                ),
            });
        }
        let indices = Self::triangulate(&vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn square(half_length: f32) -> Self {
        Self::cuboid(Vector2::new(half_length, half_length))
    }

    pub fn cuboid(half_extents: Vector2<f32>) -> Self {
        let vertices = vec![
            // Top left
            Vertex2D::new(
                Vector2::new(-half_extents.x, half_extents.y),
                Vector2::new(0.0, 0.0),
            ),
            // Bottom left
            Vertex2D::new(
                Vector2::new(-half_extents.x, -half_extents.y),
                Vector2::new(0.0, 1.0),
            ),
            // Bottom right
            Vertex2D::new(
                Vector2::new(half_extents.x, -half_extents.y),
                Vector2::new(1.0, 1.0),
            ),
            // Top right
            Vertex2D::new(
                Vector2::new(half_extents.x, half_extents.y),
                Vector2::new(1.0, 0.0),
            ),
        ];
        let indices = Vec::from(Self::CUBOID_INDICES);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn from_aabb(aabb: AABB) -> Self {
        let vertices = vec![
            Vertex2D::new(Vector2::new(aabb.min.x, aabb.max.y), Vector2::new(0.0, 0.0)),
            Vertex2D::new(Vector2::new(aabb.min.x, aabb.min.y), Vector2::new(0.0, 1.0)),
            Vertex2D::new(Vector2::new(aabb.max.x, aabb.min.y), Vector2::new(1.0, 1.0)),
            Vertex2D::new(Vector2::new(aabb.max.x, aabb.max.y), Vector2::new(1.0, 0.0)),
        ];
        let indices = Vec::from(Self::CUBOID_INDICES);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn aabb(&self) -> AABB {
        AABB::from_vertices(&self.vertices)
    }

    pub fn triangle(a: Vector2<f32>, b: Vector2<f32>, c: Vector2<f32>) -> Self {
        let ccw = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        let vertices = if ccw > 0.0 {
            vec![a, b, c]
        } else {
            vec![c, b, a]
        };
        let vertices = Self::create_tex(vertices);
        let indices = Vec::from(Self::TRIANGLE_INDICES);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn segment(a: Vector2<f32>, b: Vector2<f32>, half_thickness: f32) -> Self {
        let d = b - a;
        let l = (d.x.powi(2) + d.y.powi(2)).sqrt();
        let r = half_thickness / l;
        let da = d * r;

        let vertices = vec![
            Vector2::new(a.x - da.x, a.y + da.y),
            Vector2::new(a.x + da.x, a.y - da.y),
            Vector2::new(b.x + da.x, b.y - da.y),
            Vector2::new(b.x - da.x, b.y + da.y),
        ];
        let vertices = Self::create_tex(vertices);
        let indices = Self::triangulate(&vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn convex_polygon(vertices: Vec<Vector2<f32>>) -> Self {
        let vertices = Self::create_tex(vertices);
        let indices = Self::triangulate(&vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn rounded(
        inner: Self,
        direction: RoundingDirection,
        border_radius: f32,
        resolution: u32,
    ) -> Self {
        let v = inner.vertices.iter().map(|v| v.pos).collect();
        let border = border_radius;

        fn det(v0: Vector2<f32>, v1: Vector2<f32>) -> f32 {
            return v0.x * v1.y - v0.y * v1.x;
        }

        fn sign(n: f32) -> f32 {
            return ((n > 0.0) as i32 - (n < 0.0) as i32) as f32;
        }

        struct WrapIter<'a> {
            len: usize,
            counter: usize,
            vertices: &'a Vec<Vector2<f32>>,
        }

        impl<'a> WrapIter<'a> {
            pub fn new(vertices: &'a Vec<Vector2<f32>>) -> WrapIter<'a> {
                Self {
                    len: vertices.len() - 1,
                    counter: 0,
                    vertices,
                }
            }
        }

        impl<'a> Iterator for WrapIter<'a> {
            type Item = (usize, usize, Vector2<f32>, Vector2<f32>);
            fn next(&mut self) -> Option<Self::Item> {
                let i = self.counter;
                self.counter += 1;
                if i < self.len {
                    return Some((i, i + 1, self.vertices[i], self.vertices[i + 1]));
                } else if i == self.len {
                    return Some((self.len, 0, self.vertices[self.len], self.vertices[0]));
                }
                return None;
            }
        }

        let factor = -1.0;
        let ccw_left = Matrix2::from(Rotation2::new(factor * FRAC_PI_2));

        let n: Vec<Vector2<f32>> = WrapIter::new(&v)
            .map(|(__, _, v0, v1)| (ccw_left * (v1 - v0).normalize() * border))
            .collect();

        let mut a: Vec<f32> = WrapIter::new(&n)
            .map(|(__, _, n0, n1)| n0.angle(&n1))
            .collect();

        let d: Vec<f32> = WrapIter::new(&n)
            .map(|(__, _, n0, n1)| sign(det(n0, n1)))
            .collect();

        let s: Vec<u32> = a.iter().map(|_| resolution).collect();
        let mut o: Vec<Option<Vector2<f32>>> = v.iter().map(|_| None).collect();
        let mut v_prime = v.clone();

        for (i, j, _, v1) in WrapIter::new(&v) {
            let h = border / (a[i] / 2.0).cos();
            let v_h = ((n[i] + n[j]) * -1.0).normalize() * h;
            match direction {
                RoundingDirection::Inward => {
                    v_prime[j] = v1 + v_h;
                }
                RoundingDirection::Outward => {
                    v_prime[j] = v1;
                }
            }
            o[j] = Some(v_prime[j] - v_h * (factor * d[i] + 1.0));
        }
        for (i, j, _, _) in WrapIter::new(&v) {
            if s[i] > 0 {
                a[i] /= s[i] as f32
            } else {
                o[j] = Some(v[j] + n[i] * factor * d[i]);
            }
        }

        let mut index = 0;
        let mut v_new: Vec<Option<Vector2<f32>>> = (0..(v_prime.len() as u32
            + s.iter().sum::<u32>()))
            .map(|_| None)
            .collect();

        for (i, j, _, _) in WrapIter::new(&v_prime) {
            let m = Matrix2::from(Rotation2::new(d[i] * a[i]));
            let mut step = n[i] * (-factor * d[i]);
            let anchor = o[j];
            v_new[index] = Some(anchor.unwrap() + step);
            index += 1;
            for _ in 0..s[i] {
                step = m * step;
                v_new[index] = Some(anchor.unwrap() + step);
                index += 1;
            }
        }

        let vertices = v_new.into_iter().map(|v| v.unwrap()).collect();
        let vertices = Self::create_tex(vertices);
        let indices = Self::triangulate(&vertices);
        Self {
            vertices,
            indices,
            ..inner
        }
    }

    pub fn star(corners: u32, inner_radius: f32, outer_radius: f32) -> Self {
        let a = PI / corners as f32;
        let v_count = 2 * corners as usize;
        let mut vertices = Vec::with_capacity(v_count);
        let mut indices = vec![];
        for i in 0..v_count {
            let r = if i % 2 == 1 {
                inner_radius
            } else {
                let prev = if i as i32 - 1 < 0 { v_count - 1 } else { i - 1 } as u32;
                let next = if i + 1 >= v_count { 1 } else { i + 1 } as u32;
                indices.push(Index::new(0, next, prev));
                indices.push(Index::new(i as u32, prev, next));
                outer_radius
            };
            vertices.push(Vector2::new(
                r * (FRAC_PI_2 - a * i as f32).cos(),
                r * (FRAC_PI_2 - a * i as f32).sin(),
            ));
        }
        vertices.push(Vector2::new(0.0, 0.0));
        let vertices = Self::create_tex(vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn custom(vertices: Vec<Vertex2D>, indices: Vec<Index>) -> Self {
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn compound(shapes: Vec<Self>) -> Self {
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut offset = 0;
        for shape in shapes {
            let shape = shape.apply();
            vertices.extend(shape.vertices);
            let len = shape.indices.len() as u32;
            for index in shape.indices {
                indices.push(Index {
                    a: index.a + offset,
                    b: index.b + offset,
                    c: index.c + offset,
                });
            }
            offset += len + 1;
        }
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    #[cfg(feature = "physics")]
    pub fn from_collider_shape(shape: &dyn Shape, resolution: u32, half_thickness: f32) -> Self {
        match shape.as_typed_shape() {
            TypedShape::Ball(ball) => {
                return Self::ball(ball.radius, resolution);
            }
            TypedShape::Cuboid(cuboid) => {
                return Self::cuboid(cuboid.half_extents.into());
            }
            TypedShape::Capsule(capsule) => {
                return Self::capsule(capsule.radius, capsule.half_height(), resolution);
            }
            TypedShape::Segment(segmenet) => {
                return Self::segment(segmenet.a.coords, segmenet.b.coords, half_thickness)
            }
            TypedShape::Triangle(triangle) => {
                return Self::triangle(triangle.a.coords, triangle.b.coords, triangle.c.coords);
            }
            TypedShape::ConvexPolygon(convex_polygon) => {
                let vertices = convex_polygon.points().iter().map(|p| p.coords).collect();
                return Self::convex_polygon(vertices);
            }
            TypedShape::RoundCuboid(round_cuboid) => {
                return Self::rounded(
                    Self::cuboid(round_cuboid.inner_shape.half_extents.into()),
                    RoundingDirection::Outward,
                    round_cuboid.border_radius,
                    resolution,
                );
            }
            TypedShape::RoundTriangle(round_triangle) => {
                let inner = round_triangle.inner_shape;
                return Self::rounded(
                    Self::triangle(inner.a.coords, inner.b.coords, inner.c.coords),
                    RoundingDirection::Outward,
                    round_triangle.border_radius,
                    resolution,
                );
            }
            TypedShape::RoundConvexPolygon(round_convex_polygon) => {
                let inner = &round_convex_polygon.inner_shape;
                let vertices = inner.points().iter().map(|p| p.coords).collect();
                return Self::rounded(
                    Self::convex_polygon(vertices),
                    RoundingDirection::Outward,
                    round_convex_polygon.border_radius,
                    resolution,
                );
            }
            TypedShape::Compound(compound) => {
                let builders = compound
                    .shapes()
                    .iter()
                    .map(|s| {
                        Self::from_collider_shape(s.1.as_ref(), resolution, half_thickness)
                            .vertex_position(s.0)
                    })
                    .collect();

                return Self::compound(builders);
            }
            TypedShape::TriMesh(tri_mesh) => {
                let builders = tri_mesh
                    .triangles()
                    .map(|s| Self::triangle(s.a.coords, s.b.coords, s.c.coords))
                    .collect();
                return Self::compound(builders);
            }
            TypedShape::Polyline(poly_line) => {
                let builders = poly_line
                    .segments()
                    .map(|s| Self::segment(s.a.coords, s.b.coords, half_thickness))
                    .collect();
                return Self::compound(builders);
            }
            TypedShape::Custom(_) | TypedShape::HalfSpace(_) | TypedShape::HeightField(_) => {
                panic!("Unsupported collider shape!");
            }
        };
    }

    /// Triangulation of vertices
    pub fn triangulate(vertices: &Vec<Vertex2D>) -> Vec<Index> {
        use delaunator::{triangulate, Point};

        let points: Vec<Point> = vertices
            .iter()
            .rev()
            .map(|v| Point {
                x: v.pos.x as f64,
                y: v.pos.y as f64,
            })
            .collect();
        let t = triangulate(&points);
        let mut indices = vec![];
        for i in 0..t.len() {
            indices.push(Index::new(
                t.triangles[3 * i] as u32,
                t.triangles[3 * i + 1] as u32,
                t.triangles[3 * i + 2] as u32,
            ));
        }
        return indices;
    }

    /// Generates the texture coordinates
    pub fn create_tex(vertices: Vec<Vector2<f32>>) -> Vec<Vertex2D> {
        let mut min_x = vertices[0].x;
        let mut max_x = vertices[0].x;
        let mut min_y = vertices[0].y;
        let mut max_y = vertices[0].y;
        for i in 1..vertices.len() {
            let v = vertices[i];
            if v.x < min_x {
                min_x = v.x;
            }
            if v.x > max_x {
                max_x = v.x;
            }

            if v.y < min_y {
                min_y = v.y;
            }
            if v.y > max_y {
                max_y = v.y;
            }
        }
        let size = Vector2::new(max_x - min_x, max_y - min_y);
        let mut result = vec![];
        for v in vertices {
            let delta_x = v.x - min_x;
            let ratio_x = delta_x / size.x;
            let delta_y = max_y - v.y;
            let ratio_y = delta_y / size.y;
            let tex = Vector2::new(ratio_x, ratio_y);
            result.push(Vertex2D::new(v, tex));
        }
        return result;
    }

    pub fn vertex_scale(mut self, scale: Vector2<f32>) -> Self {
        self.vertex_scale = scale;
        self
    }

    pub fn vertex_position(mut self, position: Isometry2<f32>) -> Self {
        self.vertex_offset = position;
        self
    }

    pub fn vertex_rotation(mut self, rotation: Rotation2<f32>) -> Self {
        self.vertex_offset.rotation = rotation;
        self
    }

    pub fn vertex_translation(mut self, translation: Vector2<f32>) -> Self {
        self.vertex_offset.translation.vector = translation;
        self
    }

    pub fn vertex_rotation_axis(mut self, rotation_axis: Vector2<f32>) -> Self {
        self.vertex_rotation_axis = rotation_axis;
        self
    }

    pub fn tex_coord_scale(mut self, scale: Vector2<f32>) -> Self {
        self.tex_coord_scale = scale;
        self
    }

    pub fn tex_coord_position(mut self, position: Isometry2<f32>) -> Self {
        self.tex_coord_offset = position;
        self
    }

    pub fn tex_coord_rotation(mut self, rotation: Rotation2<f32>) -> Self {
        self.tex_coord_offset.rotation = rotation;
        self
    }

    pub fn tex_coord_translation(mut self, translation: Vector2<f32>) -> Self {
        self.tex_coord_offset.translation.vector = translation;
        self
    }

    pub fn tex_coord_rotation_axis(mut self, rotation_axis: Vector2<f32>) -> Self {
        self.tex_coord_rotation_axis = rotation_axis;
        self
    }

    pub fn vertex_size(&self) -> wgpu::BufferAddress {
        mem::size_of::<Vertex2D>() as u64
    }

    pub fn apply(mut self) -> Self {
        Self::compute_modifed_vertices(
            &mut self.vertices,
            self.vertex_offset,
            self.tex_coord_offset,
            self.vertex_scale,
            self.tex_coord_scale,
            self.vertex_rotation_axis,
            self.tex_coord_rotation_axis,
        );
        Self {
            vertices: self.vertices,
            indices: self.indices,
            ..Default::default()
        }
    }

    pub fn compute_modifed_vertices(
        vertices: &mut Vec<Vertex2D>,
        vertex_offset: Isometry2<f32>,
        tex_coord_offset: Isometry2<f32>,
        vertex_scale: Vector2<f32>,
        tex_coord_scale: Vector2<f32>,
        vertex_rotation_axis: Vector2<f32>,
        tex_coord_rotation_axis: Vector2<f32>,
    ) {
        if vertex_scale != Self::DEFAULT_SCALE {
            for v in vertices.iter_mut() {
                v.pos.x *= vertex_scale.x;
                v.pos.y *= vertex_scale.y;
            }
        }

        let angle = vertex_offset.rotation.angle();
        if angle != Self::DEFAULT_ROTATION {
            for v in vertices.iter_mut() {
                let delta = v.pos - vertex_rotation_axis;
                v.pos = vertex_rotation_axis + vertex_offset.rotation * delta;
            }
        }

        if vertex_offset.translation.vector != Self::DEFAULT_OFFSET {
            for v in vertices.iter_mut() {
                v.pos += vertex_offset.translation.vector;
            }
        }

        if tex_coord_scale != Self::DEFAULT_SCALE {
            for v in vertices.iter_mut() {
                v.tex.x *= tex_coord_scale.x;
                v.tex.y *= tex_coord_scale.y;
            }
        }

        let angle = tex_coord_offset.rotation.angle();
        if angle != Self::DEFAULT_ROTATION {
            for v in vertices.iter_mut() {
                let delta = v.tex - tex_coord_rotation_axis;
                v.tex = tex_coord_rotation_axis + tex_coord_offset.rotation * delta;
            }
        }

        if tex_coord_offset.translation.vector != Self::DEFAULT_OFFSET {
            for v in vertices.iter_mut() {
                v.tex += tex_coord_offset.translation.vector;
            }
        }
    }
}

impl Default for MeshBuilder2D {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            indices: Default::default(),
            vertex_offset: Isometry2::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            vertex_scale: Self::DEFAULT_SCALE,
            vertex_rotation_axis: Vector2::new(0.0, 0.0),
            tex_coord_offset: Isometry2::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            tex_coord_scale: Self::DEFAULT_SCALE,
            tex_coord_rotation_axis: Vector2::new(0.5, 0.5),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex3D {
    pub pos: Vector3<f32>,
    pub tex: Vector2<f32>,
    pub normal: Vector3<f32>,
}

impl Vertex3D {
    pub const fn new(pos: Vector3<f32>, tex: Vector2<f32>, normal: Vector3<f32>) -> Self {
        Vertex3D { pos, tex, normal }
    }
}

impl Vertex for Vertex3D {
    const SIZE: u64 = mem::size_of::<Self>() as u64;
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x3,
    ];
}

pub struct MeshBuilder3D {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<Index>,
}

impl MeshBuilder for MeshBuilder3D {
    type Vertex = Vertex3D;

    fn indices<'a>(&'a self) -> &'a [Index] {
        &self.indices
    }

    fn vertices<'a>(&'a self) -> &'a [Self::Vertex] {
        &self.vertices
    }
}

impl MeshBuilder3D {
    pub fn plane(top_left: Vector3<f32>, bottom_right: Vector3<f32>) -> Self {
        let top_right = Vector3::new(bottom_right.x, top_left.y, 0.0);
        let bottom_left = Vector3::new(top_left.x, bottom_right.y, 0.0);
        Self {
            vertices: vec![
                // Top left
                Vertex3D::new(top_left, Vector2::new(0.0, 0.0), Default::default()),
                // Bottom left
                Vertex3D::new(bottom_left, Vector2::new(0.0, 1.0), Default::default()),
                // Bottom right
                Vertex3D::new(bottom_right, Vector2::new(1.0, 1.0), Default::default()),
                // Top right
                Vertex3D::new(top_right, Vector2::new(1.0, 0.0), Default::default()),
            ],
            indices: vec![Index::new(0, 1, 2), Index::new(0, 2, 3)],
        }
    }

    pub fn cube(half_size: Vector3<f32>) -> Self {
        Self {
            vertices: vec![
                Vertex3D::new(
                    Vector3::new(-half_size.x, -half_size.y, -half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(half_size.x, -half_size.y, -half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(half_size.x, half_size.y, -half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(-half_size.x, half_size.y, -half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(-half_size.x, -half_size.y, half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(half_size.x, -half_size.y, half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(half_size.x, half_size.y, half_size.z),
                    Default::default(),
                    Default::default(),
                ),
                Vertex3D::new(
                    Vector3::new(-half_size.x, half_size.y, half_size.z),
                    Default::default(),
                    Default::default(),
                ),
            ],
            indices: vec![
                Index::new(0, 1, 2),
                Index::new(0, 2, 3),
                Index::new(1, 5, 6),
                Index::new(1, 6, 2),
                Index::new(5, 4, 7),
                Index::new(5, 7, 6),
                Index::new(4, 0, 3),
                Index::new(4, 3, 7),
                Index::new(3, 2, 6),
                Index::new(3, 6, 7),
                Index::new(4, 5, 1),
                Index::new(4, 1, 0),
            ],
        }
    }
}

#[derive(Debug)]
pub struct Mesh<V: Vertex> {
    vertex_amount: u32,
    index_amount: u32,
    vertex_buffer_size: wgpu::BufferAddress,
    index_buffer_size: wgpu::BufferAddress,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    marker: PhantomData<V>,
}

impl<V: Vertex> Mesh<V> {
    pub fn new(gpu: &Gpu, builder: impl MeshBuilder<Vertex = V>) -> Self {
        let vertices = builder.vertices();
        let indices = builder.indices();
        let vertices_slice = bytemuck::cast_slice(&vertices);
        let indices_slice = bytemuck::cast_slice(&indices);
        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: vertices_slice,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("index_buffer"),
                contents: indices_slice,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

        Mesh {
            vertex_buffer_size: vertices_slice.len() as wgpu::BufferAddress,
            index_buffer_size: indices_slice.len() as wgpu::BufferAddress,
            vertex_buffer,
            index_buffer,
            vertex_amount: vertices.len() as u32,
            index_amount: indices.len() as u32 * 3,
            marker: PhantomData,
        }
    }

    pub fn write(&mut self, gpu: &Gpu, builder: impl MeshBuilder<Vertex = V>) {
        let vertices = builder.vertices();
        let indices = builder.indices();
        self.write_indices(gpu, &indices);
        self.write_vertices(gpu, &vertices);
    }

    pub fn write_indices(&mut self, gpu: &Gpu, indices: &[Index]) {
        let indices_slice = bytemuck::cast_slice(&indices[..]);
        let new_size = indices_slice.len() as wgpu::BufferAddress;
        if new_size > self.index_buffer_size {
            self.index_buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("index_buffer"),
                    contents: indices_slice,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
        } else {
            gpu.queue.write_buffer(&self.index_buffer, 0, indices_slice);
        }
        self.index_buffer_size = new_size;
        self.index_amount = indices.len() as u32 * 3;
    }

    pub fn write_vertices(&mut self, gpu: &Gpu, vertices: &[V]) {
        let vertices_slice = bytemuck::cast_slice(&vertices[..]);
        let new_size = vertices_slice.len() as wgpu::BufferAddress;
        if new_size > self.vertex_buffer_size {
            self.vertex_buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vertex_buffer"),
                    contents: vertices_slice,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        } else {
            gpu.queue
                .write_buffer(&self.vertex_buffer, 0, vertices_slice);
        }
        self.vertex_buffer_size = new_size;
        self.vertex_amount = vertices.len() as u32;
    }

    pub fn vertex_buffer(&self) -> wgpu::BufferSlice {
        self.vertex_buffer.slice(..self.vertex_buffer_size)
    }

    pub fn index_buffer(&self) -> wgpu::BufferSlice {
        self.index_buffer.slice(..self.index_buffer_size)
    }

    pub fn index_amount(&self) -> u32 {
        self.index_amount
    }

    pub fn vertex_amount(&self) -> u32 {
        self.vertex_amount
    }

    pub fn vertex_buffer_size(&self) -> wgpu::BufferAddress {
        self.vertex_buffer_size
    }

    pub fn index_buffer_size(&self) -> wgpu::BufferAddress {
        self.index_buffer_size
    }

    pub fn vertex_size(&self) -> wgpu::BufferAddress {
        V::SIZE
    }

    pub(crate) fn buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RoundingDirection {
    Inward,
    Outward,
}

pub struct ModelBuilder {
    pub meshes: Vec<tobj::Model>,
    pub sprites: Vec<Vec<u8>>,
}

impl ModelBuilder {
    pub fn file(path: &str) -> Self {
        let obj_text = load_string(path).unwrap();
        let obj_cursor = Cursor::new(&obj_text);
        let mut obj_reader = BufReader::new(obj_cursor);
        let mut path_buf: std::path::PathBuf = path.into();
        path_buf.pop();

        let (obj_meshes, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                let mat_text = load_string(path_buf.join(p)).unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
            },
        )
        .unwrap();

        let mut sprites = Vec::new();
        for m in obj_materials.unwrap() {
            sprites.push(load_bytes(m.diffuse_texture.unwrap()).unwrap());
        }

        Self {
            meshes: obj_meshes,
            sprites,
        }
    }

    pub fn bytes(obj: &str, mtl: &[(&str, &str)], materials: &[(&str, &[u8])]) -> Self {
        let obj_cursor = Cursor::new(obj);
        let mut obj_reader = BufReader::new(obj_cursor);

        let (obj_meshes, obj_materials) = tobj::load_obj_buf(
            &mut obj_reader,
            &tobj::LoadOptions {
                triangulate: true,
                single_index: true,
                ..Default::default()
            },
            |p| {
                let mtl = mtl
                    .iter()
                    .find(|(key, _)| *key == p.to_str().unwrap())
                    .unwrap();
                tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mtl.1)))
            },
        )
        .unwrap();

        let mut sprites = Vec::new();
        for m in obj_materials.unwrap() {
            let material = materials
                .iter()
                .find(|(key, _)| *key == m.diffuse_texture.clone().unwrap())
                .unwrap();
            sprites.push(material.1.to_vec());
        }

        Self {
            meshes: obj_meshes,
            sprites,
        }
    }
}

pub struct Model {
    pub meshes: Vec<(Option<usize>, Mesh3D)>,
    pub sprites: Vec<Sprite>,
}

impl Model {
    pub fn new(gpu: &Gpu, builder: ModelBuilder) -> Self {
        let sprites = builder
            .sprites
            .into_iter()
            .map(|m| gpu.create_sprite(SpriteBuilder::bytes(&m)))
            .collect::<Vec<_>>();
        let meshes = builder
            .meshes
            .into_iter()
            .map(|m| {
                let vertices = (0..m.mesh.positions.len() / 3)
                    .map(|i| Vertex3D {
                        pos: Vector3::new(
                            m.mesh.positions[i * 3],
                            m.mesh.positions[i * 3 + 1],
                            m.mesh.positions[i * 3 + 2],
                        ),
                        tex: Vector2::new(
                            *m.mesh.texcoords.get(i * 2).unwrap_or(&0.0),
                            *m.mesh.texcoords.get(i * 2 + 1).unwrap_or(&0.0),
                        ),
                        normal: Vector3::new(
                            *m.mesh.normals.get(i * 3).unwrap_or(&0.0),
                            *m.mesh.normals.get(i * 3 + 1).unwrap_or(&0.0),
                            *m.mesh.normals.get(i * 3 + 2).unwrap_or(&0.0),
                        ),
                    })
                    .collect::<Vec<_>>();
                (
                    m.mesh.material_id,
                    gpu.create_mesh(MeshBuilder3D {
                        vertices,
                        indices: Index::from_vec(m.mesh.indices),
                    }),
                )
            })
            .collect::<Vec<_>>();

        Self { meshes, sprites }
    }
}
