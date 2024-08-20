use std::{
    f32::consts::{FRAC_PI_2, PI},
    fmt::Debug,
    mem,
};
use wgpu::util::DeviceExt;

#[cfg(feature = "physics")]
use crate::physics::{Shape, TypedShape};
use crate::{
    graphics::{Color, Gpu},
    math::{Isometry2, Matrix2, Rotation2, Vector2, Vector3, AABB},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpriteArrayCoordinates {
    pub coords: Vector2<f32>,
    pub index: u32,
}

pub type SpriteCoordinates = Vector2<f32>;
pub type Index = u32;

pub type SpriteVertex2D = Vertex2D<SpriteCoordinates>;
pub type SpriteArrayVertex2D = Vertex2D<SpriteArrayCoordinates>;
pub type ColorVertex2D = Vertex2D<Color>;
pub type PositionVertex2D = Vertex2D<()>;

pub type SpriteMeshBuilder2D = MeshBuilder2D<SpriteVertex2D>;
pub type SpriteArrayMeshBuilder2D = MeshBuilder2D<SpriteArrayVertex2D>;
pub type ColorMeshBuilder2D = MeshBuilder2D<ColorVertex2D>;
pub type PositionMeshBuilder2D = MeshBuilder2D<PositionVertex2D>;

pub type SpriteMesh2D = Mesh<SpriteVertex2D>;
pub type SpriteArrayMesh2D = Mesh<SpriteArrayVertex2D>;
pub type ColorMesh2D = Mesh<ColorVertex2D>;
pub type PositionMesh2D = Mesh<PositionVertex2D>;
pub type Mesh3D = Mesh<Vertex3D>;

pub trait MeshBuilder {
    type Vertex;
    fn indices(&self) -> &[Index];
    fn vertices(&self) -> &[Self::Vertex];
}

pub trait Vertex: bytemuck::Pod + bytemuck::Zeroable + Send + Sync + Debug {
    const ATTRIBUTES: &'static [wgpu::VertexFormat];
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
}

impl<V: Vertex> MeshBuilder for (Vec<V>, Vec<Index>) {
    type Vertex = V;

    fn indices(&self) -> &[Index] {
        &self.1
    }
    fn vertices(&self) -> &[Self::Vertex] {
        &self.0
    }
}

impl<V: Vertex> MeshBuilder for (&[V], &[Index]) {
    type Vertex = V;

    fn indices(&self) -> &[Index] {
        self.1
    }
    fn vertices(&self) -> &[Self::Vertex] {
        self.0
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Zeroable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Vertex2D<D> where D: bytemuck::Pod + Default {
    pub pos: Vector2<f32>,
    pub data: D,
}
unsafe impl<D: bytemuck::Pod + Default> bytemuck::Pod for Vertex2D<D> {}

impl<D: bytemuck::Pod + Default> Vertex2D<D> {
    pub fn offset(mut self, offset: Isometry2<f32>) -> Self {
        let rotation = offset.rotation;
        let translation = offset.translation.vector;
        self.pos = rotation * self.pos;
        self.pos += translation;
        self
    }
}

pub trait VertexPosition2D {
    fn pos(&self) -> &Vector2<f32>;
    fn pos_mut(&mut self) -> &mut Vector2<f32>;
    fn with_pos(&self, pos: Vector2<f32>) -> Self;
}

impl<D: bytemuck::Pod + Default> VertexPosition2D for Vertex2D<D> {
    fn pos(&self) -> &Vector2<f32> {
        &self.pos
    }

    fn pos_mut(&mut self) -> &mut Vector2<f32> {
        &mut self.pos
    }

    fn with_pos(&self, pos: Vector2<f32>) -> Self {
        Self {
            pos,
            data: self.data,
        }
    }
}

pub trait BaseVertex2D: bytemuck::Pod + VertexPosition2D + VertexPosition2D {
    fn create_data(vertices: Vec<Vector2<f32>>) -> Vec<Self>;
}

impl BaseVertex2D for SpriteVertex2D {
    fn create_data(vertices: Vec<Vector2<f32>>) -> Vec<Self> {
        let mut min_x = vertices[0].x;
        let mut max_x = vertices[0].x;
        let mut min_y = vertices[0].y;
        let mut max_y = vertices[0].y;
        for v in vertices.iter().skip(1) {
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
        result
    }
}

impl BaseVertex2D for SpriteArrayVertex2D {
    fn create_data(vertices: Vec<Vector2<f32>>) -> Vec<Self> {
        let sprite_vertices = SpriteVertex2D::create_data(vertices);
        sprite_vertices
            .into_iter()
            .map(|v| {
                Vertex2D::new(
                    v.pos,
                    SpriteArrayCoordinates {
                        coords: v.data,
                        index: 0,
                    },
                )
            })
            .collect()
    }
}

impl BaseVertex2D for ColorVertex2D {
    fn create_data(vertices: Vec<Vector2<f32>>) -> Vec<Self> {
        vertices
            .into_iter()
            .map(|v| Vertex2D::new(v, Color::RED))
            .collect()
    }
}

impl BaseVertex2D for PositionVertex2D {
    fn create_data(vertices: Vec<Vector2<f32>>) -> Vec<Self> {
        let mut result = vec![];
        for v in vertices {
            result.push(Vertex2D::new(v, ()));
        }
        result
    }
}

impl<D: bytemuck::Pod + Default> Vertex2D<D> {
    pub const fn new(pos: Vector2<f32>, data: D) -> Self {
        Vertex2D { pos, data }
    }
}

impl Vertex for PositionVertex2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[wgpu::VertexFormat::Float32x2];
}

impl Vertex for SpriteVertex2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] =
        &[wgpu::VertexFormat::Float32x2, wgpu::VertexFormat::Float32x2];
}

impl Vertex for SpriteArrayVertex2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Uint32,
    ];
}

impl Vertex for ColorVertex2D {
    const ATTRIBUTES: &'static [wgpu::VertexFormat] =
        &[wgpu::VertexFormat::Float32x2, wgpu::VertexFormat::Float32x4];
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MeshBuilder2D<V: BaseVertex2D> {
    pub vertices: Vec<V>,
    pub indices: Vec<Index>,
}

impl<V: BaseVertex2D> MeshBuilder for MeshBuilder2D<V> {
    type Vertex = V;

    fn indices(&self) -> &[Index] {
        &self.indices
    }
    fn vertices(&self) -> &[Self::Vertex] {
        &self.vertices
    }
}

impl<V: BaseVertex2D> MeshBuilder2D<V> {
    pub const TRIANGLE_INDICES: [Index; 3] = [0, 1, 2];
    pub const CUBOID_INDICES: [Index; 6] = [0, 1, 2, 2, 3, 0];

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

            vertices.push(pos);
        }
        let indices = Self::triangulate(&vertices);
        let vertices = V::create_data(vertices);
        Self { vertices, indices }
    }

    pub fn square(half_length: f32) -> Self {
        Self::cuboid(Vector2::new(half_length, half_length))
    }

    pub fn cuboid(half_extents: Vector2<f32>) -> Self {
        let vertices = vec![
            Vector2::new(-half_extents.x, half_extents.y),
            Vector2::new(-half_extents.x, -half_extents.y),
            Vector2::new(half_extents.x, -half_extents.y),
            Vector2::new(half_extents.x, half_extents.y),
        ];
        let indices = Vec::from(Self::CUBOID_INDICES);
        let vertices = V::create_data(vertices);
        Self { vertices, indices }
    }

    pub fn from_aabb(aabb: AABB) -> Self {
        let vertices = vec![
            Vector2::new(aabb.min().x, aabb.max().y),
            Vector2::new(aabb.min().x, aabb.min().y),
            Vector2::new(aabb.max().x, aabb.min().y),
            Vector2::new(aabb.max().x, aabb.max().y),
        ];
        let indices = Vec::from(Self::CUBOID_INDICES);
        let vertices = V::create_data(vertices);
        Self { vertices, indices }
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
        let vertices = V::create_data(vertices);
        let indices = Vec::from(Self::TRIANGLE_INDICES);
        Self { vertices, indices }
    }

    pub fn segment(start: Vector2<f32>, end: Vector2<f32>, half_thickness: f32) -> Self {
        let direction = end - start;
        let normal = Vector2::new(-direction[1], direction[0]).normalize();

        let offset1 = normal * half_thickness;
        let offset2 = -normal * half_thickness;

        let vertices = vec![
            Vector2::new(start[0] + offset1[0], start[1] + offset1[1]),
            Vector2::new(end[0] + offset1[0], end[1] + offset1[1]),
            Vector2::new(end[0] + offset2[0], end[1] + offset2[1]),
            Vector2::new(start[0] + offset2[0], start[1] + offset2[1]),
        ];
        let indices = Self::triangulate(&vertices);
        let vertices = V::create_data(vertices);
        Self { vertices, indices }
    }

    pub fn convex_polygon(vertices: Vec<Vector2<f32>>) -> Self {
        let indices = Self::triangulate(&vertices);
        let vertices = V::create_data(vertices);
        Self { vertices, indices }
    }

    pub fn rounded(
        inner: Self,
        direction: RoundingDirection,
        border_radius: f32,
        resolution: u32,
    ) -> Self {
        let v = inner.vertices.iter().map(|v| *v.pos()).collect();
        let border = border_radius;

        fn det(v0: Vector2<f32>, v1: Vector2<f32>) -> f32 {
            v0.x * v1.y - v0.y * v1.x
        }

        fn sign(n: f32) -> f32 {
            ((n > 0.0) as i32 - (n < 0.0) as i32) as f32
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
                None
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

        let vertices = v_new.into_iter().map(|v| v.unwrap()).collect::<Vec<_>>();
        let indices = Self::triangulate(&vertices);
        let vertices = V::create_data(vertices);
        Self {
            vertices,
            indices,
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
                indices.extend([0, next, prev]);
                indices.extend([i as u32, prev, next]);
                outer_radius
            };
            vertices.push(Vector2::new(
                r * (FRAC_PI_2 - a * i as f32).cos(),
                r * (FRAC_PI_2 - a * i as f32).sin(),
            ));
        }
        vertices.push(Vector2::new(0.0, 0.0));
        let vertices = V::create_data(vertices);
        Self { vertices, indices }
    }

    pub fn custom(vertices: Vec<V>, indices: Vec<Index>) -> Self {
        Self { vertices, indices }
    }

    pub fn compound(shapes: &[Self]) -> Self {
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut offset = 0;
        for shape in shapes {
            vertices.extend(&shape.vertices);
            for index in &shape.indices {
                indices.push(index + offset);
            }
            offset += shape.vertices.len() as u32;
        }
        Self { vertices, indices }
    }

    #[cfg(feature = "physics")]
    pub fn from_collider_shape(shape: &dyn Shape, resolution: u32, half_thickness: f32) -> Self {
        match shape.as_typed_shape() {
            TypedShape::Ball(ball) => Self::ball(ball.radius, resolution),
            TypedShape::Cuboid(cuboid) => Self::cuboid(cuboid.half_extents),
            TypedShape::Capsule(capsule) => {
                Self::capsule(capsule.radius, capsule.half_height(), resolution)
            }
            TypedShape::Segment(segmenet) => {
                Self::segment(segmenet.a.coords, segmenet.b.coords, half_thickness)
            }
            TypedShape::Triangle(triangle) => {
                Self::triangle(triangle.a.coords, triangle.b.coords, triangle.c.coords)
            }
            TypedShape::ConvexPolygon(convex_polygon) => {
                let vertices = convex_polygon.points().iter().map(|p| p.coords).collect();
                Self::convex_polygon(vertices)
            }
            TypedShape::RoundCuboid(round_cuboid) => Self::rounded(
                Self::cuboid(round_cuboid.inner_shape.half_extents),
                RoundingDirection::Outward,
                round_cuboid.border_radius,
                resolution,
            ),
            TypedShape::RoundTriangle(round_triangle) => {
                let inner = round_triangle.inner_shape;
                Self::rounded(
                    Self::triangle(inner.a.coords, inner.b.coords, inner.c.coords),
                    RoundingDirection::Outward,
                    round_triangle.border_radius,
                    resolution,
                )
            }
            TypedShape::RoundConvexPolygon(round_convex_polygon) => {
                let inner = &round_convex_polygon.inner_shape;
                let vertices = inner.points().iter().map(|p| p.coords).collect();
                Self::rounded(
                    Self::convex_polygon(vertices),
                    RoundingDirection::Outward,
                    round_convex_polygon.border_radius,
                    resolution,
                )
            }
            TypedShape::Compound(compound) => {
                let builders: Vec<_> = compound
                    .shapes()
                    .iter()
                    .map(|s| {
                        Self::from_collider_shape(s.1.as_ref(), resolution, half_thickness)
                            .apply_vertex_position(s.0)
                    })
                    .collect();

                Self::compound(&builders)
            }
            TypedShape::TriMesh(tri_mesh) => {
                let builders: Vec<_> = tri_mesh
                    .triangles()
                    .map(|s| Self::triangle(s.a.coords, s.b.coords, s.c.coords))
                    .collect();
                Self::compound(&builders)
            }
            TypedShape::Polyline(poly_line) => {
                let builders: Vec<_> = poly_line
                    .segments()
                    .map(|s| Self::segment(s.a.coords, s.b.coords, half_thickness))
                    .collect();
                Self::compound(&builders)
            }
            TypedShape::Custom(_) | TypedShape::HalfSpace(_) | TypedShape::HeightField(_) => {
                panic!("Unsupported collider shape!");
            }
        }
    }

    pub fn triangulate(vertices: &[Vector2<f32>]) -> Vec<Index> {
        use delaunator::{triangulate, Point};

        let points: Vec<Point> = vertices
            .iter()
            .rev()
            .map(|v| Point {
                x: v.x as f64,
                y: v.y as f64,
            })
            .collect();
        let t = triangulate(&points);
        let mut indices = vec![];
        for i in 0..t.len() {
            indices.extend([
                t.triangles[3 * i] as u32,
                t.triangles[3 * i + 1] as u32,
                t.triangles[3 * i + 2] as u32,
            ]);
        }
        indices
    }

    pub fn apply_vertex_scale(mut self, scale: Vector2<f32>) -> Self {
        for v in &mut self.vertices {
            v.pos_mut().x *= scale.x;
            v.pos_mut().y *= scale.y;
        }
        self
    }

    pub fn apply_vertex_position(self, position: Isometry2<f32>) -> Self {
        self.apply_vertex_translation(position.translation.vector)
            .apply_vertex_rotation(position.rotation, position.translation.vector)
    }

    pub fn apply_vertex_rotation(
        mut self,
        rotation: Rotation2<f32>,
        vertex_rotation_axis: Vector2<f32>,
    ) -> Self {
        for v in &mut self.vertices {
            let delta = v.pos() - vertex_rotation_axis;
            *v.pos_mut() = vertex_rotation_axis + rotation * delta;
        }
        self
    }

    pub fn apply_vertex_translation(mut self, translation: Vector2<f32>) -> Self {
        for v in &mut self.vertices {
            *v.pos_mut() += translation;
        }
        self
    }
}

impl<D: bytemuck::Pod + Default> MeshBuilder2D<Vertex2D<D>>
where
    Vertex2D<D>: BaseVertex2D,
{
    pub fn apply_data(mut self, data: D) -> Self {
        for v in &mut self.vertices {
            v.data = data;
        }
        self
    }
}

impl MeshBuilder2D<SpriteVertex2D> {
    pub fn apply_tex_coord_rotation(
        mut self,
        rotation: Rotation2<f32>,
        tex_coord_rotation_axis: Vector2<f32>,
    ) -> Self {
        for v in &mut self.vertices {
            let delta: nalgebra::Matrix<
                f32,
                nalgebra::Const<2>,
                nalgebra::Const<1>,
                nalgebra::ArrayStorage<f32, 2, 1>,
            > = v.data - tex_coord_rotation_axis;
            v.data = tex_coord_rotation_axis + rotation * delta;
        }
        self
    }

    pub fn apply_tex_coord_translation(mut self, translation: Vector2<f32>) -> Self {
        for v in &mut self.vertices {
            v.data += translation;
        }
        self
    }

    pub fn apply_tex_coord_scale(mut self, scale: Vector2<f32>) -> Self {
        for v in &mut self.vertices {
            v.data.x *= scale.x;
            v.data.y *= scale.y;
        }
        self
    }

    pub fn apply_tex_coord_position(self, position: Isometry2<f32>) -> Self {
        self.apply_tex_coord_rotation(position.rotation, Vector2::new(0.5, 0.5))
            .apply_tex_coord_translation(position.translation.vector)
    }
}

impl<V: BaseVertex2D> Default for MeshBuilder2D<V> {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            indices: Default::default(),
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
    const ATTRIBUTES: &'static [wgpu::VertexFormat] = &[
        wgpu::VertexFormat::Float32x3,
        wgpu::VertexFormat::Float32x2,
        wgpu::VertexFormat::Float32x3,
    ];
}

pub struct MeshBuilder3D {
    pub vertices: Vec<Vertex3D>,
    pub indices: Vec<Index>,
}

impl MeshBuilder for MeshBuilder3D {
    type Vertex = Vertex3D;

    fn indices(&self) -> &[Index] {
        &self.indices
    }

    fn vertices(&self) -> &[Self::Vertex] {
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
            indices: vec![0, 1, 2, 0, 2, 3],
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
            #[rustfmt::skip]
            indices: vec![
                0, 1, 2,
                0, 2, 3,
                1, 5, 6,
                1, 6, 2,
                5, 4, 7,
                5, 7, 6,
                4, 0, 3,
                4, 3, 7,
                3, 2, 6,
                3, 6, 7,
                4, 5, 1,
                4, 1, 0,
            ],
        }
    }
}

#[derive(Debug)]
pub struct Mesh<V: Vertex> {
    vertex_amount: u32,
    index_amount: u32,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub(crate) vertex_data: Vec<V>,
    pub(crate) index_data: Vec<Index>,
    pub(crate) force_update: bool,
    pub(crate) write_indices: bool,
}

impl<V: Vertex> Mesh<V> {
    pub fn new(gpu: &Gpu, builder: &dyn MeshBuilder<Vertex = V>) -> Self {
        let vertices = builder.vertices();
        let indices = builder.indices();
        let vertices_slice = bytemuck::cast_slice(vertices);
        let indices_slice = bytemuck::cast_slice(indices);
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
            vertex_buffer,
            index_buffer,
            vertex_amount: vertices.len() as u32,
            index_amount: indices.len() as u32,
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            force_update: true,
            write_indices: true
        }
    }

    pub fn empty(gpu: &Gpu, vertices: u64, indices: u64) -> Self {
        let vertex_buffer_size = V::SIZE * vertices;
        let index_buffer_size = std::mem::size_of::<u32>() as wgpu::BufferAddress * indices;
        let vertex_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex_buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: vertex_buffer_size,
            mapped_at_creation: true,
        });

        let index_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index_buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            size: index_buffer_size,
            mapped_at_creation: false,
        });

        Mesh {
            vertex_buffer,
            index_buffer,
            vertex_amount: 0,
            index_amount: 0,
            vertex_data: Vec::new(),
            index_data: Vec::new(),
            force_update: true,
            write_indices: true,
        }
    }

    pub fn write(&mut self, gpu: &Gpu, builder: &dyn MeshBuilder<Vertex = V>) {
        let vertices = builder.vertices();
        let indices = builder.indices();
        self.write_indices(gpu, indices);
        self.write_vertices(gpu, vertices);
    }

    pub fn write_indices(&mut self, gpu: &Gpu, indices: &[Index]) {
        self.write_indices_offset(gpu, indices, 0)
    }

    pub fn write_vertices(&mut self, gpu: &Gpu, vertices: &[V]) {
        self.write_vertices_offset(gpu, vertices, 0)
    }

    pub fn write_indices_offset(&mut self, gpu: &Gpu, indices: &[Index], offset: u64) {
        let indices_slice = bytemuck::cast_slice(indices);
        let new_size = indices_slice.len() as wgpu::BufferAddress;
        if new_size > self.index_buffer_capacity() {
            self.index_buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("index_buffer"),
                    contents: indices_slice,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                });
            debug_assert_eq!(new_size, self.index_buffer.size());
        } else {
            gpu.queue
                .write_buffer(&self.index_buffer, offset, indices_slice);
        }
        self.index_amount = indices.len() as u32;
    }

    pub fn write_vertices_offset(&mut self, gpu: &Gpu, vertices: &[V], offset: u64) {
        let vertices_slice = bytemuck::cast_slice(vertices);
        let new_size = vertices_slice.len() as wgpu::BufferAddress;
        if new_size > self.vertex_buffer_capacity() {
            self.vertex_buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("vertex_buffer"),
                    contents: vertices_slice,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
            debug_assert_eq!(new_size, self.vertex_buffer.size());
        } else {
            gpu.queue
                .write_buffer(&self.vertex_buffer, offset, vertices_slice);
        }
        self.vertex_amount = vertices.len() as u32;
    }

    pub fn vertex_buffer(&self) -> wgpu::BufferSlice {
        self.vertex_buffer.slice(..self.vertex_buffer_size())
    }

    pub fn index_buffer(&self) -> wgpu::BufferSlice {
        self.index_buffer.slice(..self.index_buffer_size())
    }

    pub fn index_amount(&self) -> u32 {
        self.index_amount
    }

    pub fn vertex_amount(&self) -> u32 {
        self.vertex_amount
    }

    pub fn vertex_buffer_size(&self) -> wgpu::BufferAddress {
        V::SIZE * self.vertex_amount() as u64
    }

    pub fn index_buffer_size(&self) -> wgpu::BufferAddress {
        std::mem::size_of::<u32>() as u64 * self.index_amount() as u64
    }

    pub fn vertex_buffer_capacity(&self) -> wgpu::BufferAddress {
        self.vertex_buffer.size()
    }

    pub fn index_buffer_capacity(&self) -> wgpu::BufferAddress {
        self.index_buffer.size()
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
