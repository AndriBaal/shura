#[cfg(feature = "physics")]
use crate::physics::{Shape, TypedShape};
use crate::{na::Matrix2, Gpu, Index, Isometry, Rotation, Vector, Vertex};
use crate::{CameraBuffer, AABB};
use std::f32::consts::{FRAC_PI_2, PI};
use wgpu::util::DeviceExt;

#[derive(Debug)]
/// Indexbuffer of a [Model]. This is either a 'custom' one for the [Model] or a shared one.
/// For example all rectangles have the same IndexBuffer, so we don't need to have a seperate one
/// for every Rectangle.
pub enum ModelIndexBuffer {
    Triangle,
    Cuboid,
    Custom(wgpu::Buffer),
}

impl Default for ModelBuilder {
    fn default() -> Self {
        Self {
            vertices: Default::default(),
            indices: Default::default(),
            vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            vertex_scale: Self::DEFAULT_SCALE,
            tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            tex_coord_scale: Self::DEFAULT_SCALE,
            vertex_rotation_axis: Vector::new(0.0, 0.0),
            tex_coord_rotation_axis: Vector::new(0.5, 0.5),
        }
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Builder to easily create a [Model].
pub struct ModelBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
    pub vertex_offset: Isometry<f32>,
    pub tex_coord_offset: Isometry<f32>,
    pub vertex_scale: Vector<f32>,
    pub tex_coord_scale: Vector<f32>,
    pub vertex_rotation_axis: Vector<f32>,
    pub tex_coord_rotation_axis: Vector<f32>,
}

impl ModelBuilder {
    pub const TRIANGLE_INDICES: [Index; 1] = [Index::new(0, 1, 2)];
    pub const CUBOID_INDICES: [Index; 2] = [Index::new(0, 1, 2), Index::new(2, 3, 0)];

    pub const DEFAULT_OFFSET: Vector<f32> = Vector::new(0.0, 0.0);
    pub const DEFAULT_ROTATION: f32 = 0.0;
    pub const DEFAULT_SCALE: Vector<f32> = Vector::new(1.0, 1.0);
    pub fn ball(radius: f32, resolution: u32) -> Self {
        Self::regular_polygon(radius, resolution)
    }

    pub fn capsule(radius: f32, half_height: f32, resolution: u32) -> Self {
        Self::rounded(
            ModelBuilder::cuboid(Vector::new(radius, half_height)),
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

            let pos = Vector::new(
                radius * (i / corners as f32 * 2.0 * PI).cos(),
                radius * (i / corners as f32 * 2.0 * PI).sin(),
            );

            vertices.push(Vertex {
                pos,
                tex_coords: Vector::new(
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
        Self::cuboid(Vector::new(half_length, half_length))
    }
    pub fn cuboid(half_extents: Vector<f32>) -> Self {
        let vertices = vec![
            Vertex::new(
                Vector::new(-half_extents.x, half_extents.y),
                Vector::new(0.0, 0.0),
            ),
            Vertex::new(
                Vector::new(-half_extents.x, -half_extents.y),
                Vector::new(0.0, 1.0),
            ),
            Vertex::new(
                Vector::new(half_extents.x, -half_extents.y),
                Vector::new(1.0, 1.0),
            ),
            Vertex::new(
                Vector::new(half_extents.x, half_extents.y),
                Vector::new(1.0, 0.0),
            ),
        ];
        let indices = Vec::from(Self::CUBOID_INDICES);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn triangle(a: Vector<f32>, b: Vector<f32>, c: Vector<f32>) -> Self {
        let ccw = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
        let vertices = if ccw > 0.0 {
            vec![a, b, c]
        } else {
            vec![c, b, a]
        };
        let vertices = Self::create_tex_coords(vertices);
        let indices = Vec::from(Self::TRIANGLE_INDICES);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn segment(a: Vector<f32>, b: Vector<f32>, half_thickness: f32) -> Self {
        let d = b - a;
        let l = (d.x.powi(2) + d.y.powi(2)).sqrt();
        let r = half_thickness / l;
        let da = d * r;

        let vertices = vec![
            Vector::new(a.x - da.x, a.y + da.y),
            Vector::new(a.x + da.x, a.y - da.y),
            Vector::new(b.x + da.x, b.y - da.y),
            Vector::new(b.x - da.x, b.y + da.y),
        ];
        let vertices = Self::create_tex_coords(vertices);
        let indices = Self::triangulate(&vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn convex_polygon(vertices: Vec<Vector<f32>>) -> Self {
        let vertices = Self::create_tex_coords(vertices);
        let indices = Self::triangulate(&vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }
    pub fn rounded(inner: ModelBuilder, border_radius: f32, resolution: u32) -> Self {
        let v = inner.vertices.iter().map(|v| v.pos).collect();
        let border = border_radius;

        fn det(v0: Vector<f32>, v1: Vector<f32>) -> f32 {
            return v0.x * v1.y - v0.y * v1.x;
        }

        fn sign(n: f32) -> f32 {
            return ((n > 0.0) as i32 - (n < 0.0) as i32) as f32;
        }

        struct WrapIter<'a> {
            len: usize,
            counter: usize,
            vertices: &'a Vec<Vector<f32>>,
        }

        impl<'a> WrapIter<'a> {
            pub fn new(vertices: &'a Vec<Vector<f32>>) -> WrapIter<'a> {
                Self {
                    len: vertices.len() - 1,
                    counter: 0,
                    vertices,
                }
            }
        }

        impl<'a> Iterator for WrapIter<'a> {
            type Item = (usize, usize, Vector<f32>, Vector<f32>);
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

        let ccw_left = Matrix2::from(Rotation::new(FRAC_PI_2));
        let n: Vec<Vector<f32>> = WrapIter::new(&v)
            .map(|(__, _, v0, v1)| (ccw_left * (v1 - v0).normalize() * border))
            .collect();

        let mut a: Vec<f32> = WrapIter::new(&n)
            .map(|(__, _, n0, n1)| n0.angle(&n1))
            .collect();

        let d: Vec<f32> = WrapIter::new(&n)
            .map(|(__, _, n0, n1)| sign(det(n0, n1)))
            .collect();

        let s: Vec<u32> = a.iter().map(|_| resolution).collect();
        let mut o: Vec<Option<Vector<f32>>> = v.iter().map(|_| None).collect();
        let mut v_prime = v.clone();

        for (i, j, _, v1) in WrapIter::new(&v) {
            let a_prime = (PI - a[i]) / 2.0;
            let h = border / a_prime.sin();
            let v_h = ((n[j] + n[i]) * -1.0).normalize() * h;
            v_prime[j] = v1 + v_h;
            o[j] = Some(v1 - v_h * d[i]);
        }
        for (i, j, _, _) in WrapIter::new(&v) {
            if s[i] > 0 {
                a[i] /= s[i] as f32
            } else {
                o[j] = Some(v[j] + n[i] * d[i]);
            }
        }

        let mut index = 0;
        let mut v_new: Vec<Option<Vector<f32>>> = (0..(v_prime.len() as u32
            + s.iter().sum::<u32>()))
            .map(|_| None)
            .collect();

        for (i, j, _, _) in WrapIter::new(&v_prime) {
            let m = Matrix2::from(Rotation::new(d[i] * a[i]));
            let mut step = n[i] * -d[i];
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
        let vertices = Self::create_tex_coords(vertices);
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
            vertices.push(Vector::new(
                r * (FRAC_PI_2 - a * i as f32).cos(),
                r * (FRAC_PI_2 - a * i as f32).sin(),
            ));
        }
        vertices.push(Vector::new(0.0, 0.0));
        let vertices = Self::create_tex_coords(vertices);
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn custom(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self {
            vertices,
            indices,
            ..Default::default()
        }
    }

    pub fn compound(shapes: Vec<ModelBuilder>) -> Self {
        let mut vertices = vec![];
        let mut indices = vec![];
        let mut offset = 0;
        for shape in shapes {
            let shape = shape.apply_modifiers();
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
                    ModelBuilder::cuboid(round_cuboid.inner_shape.half_extents.into()),
                    round_cuboid.border_radius,
                    resolution,
                );
            }
            TypedShape::RoundTriangle(round_triangle) => {
                let inner = round_triangle.inner_shape;
                return Self::rounded(
                    ModelBuilder::triangle(inner.a.coords, inner.b.coords, inner.c.coords),
                    round_triangle.border_radius,
                    resolution,
                );
            }
            TypedShape::RoundConvexPolygon(round_convex_polygon) => {
                let inner = &round_convex_polygon.inner_shape;
                let vertices = inner.points().iter().map(|p| p.coords).collect();
                return Self::rounded(
                    ModelBuilder::convex_polygon(vertices),
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
    pub fn triangulate(vertices: &Vec<Vertex>) -> Vec<Index> {
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
    pub fn create_tex_coords(vertices: Vec<Vector<f32>>) -> Vec<Vertex> {
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
        let size = Vector::new(max_x - min_x, max_y - min_y);
        let mut result = vec![];
        for v in vertices {
            let delta_x = v.x - min_x;
            let ratio_x = delta_x / size.x;
            let delta_y = max_y - v.y;
            let ratio_y = delta_y / size.y;
            let tex_coords = Vector::new(ratio_x, ratio_y);
            result.push(Vertex::new(v, tex_coords));
        }
        return result;
    }

    pub fn vertex_scale(mut self, scale: Vector<f32>) -> Self {
        self.vertex_scale = scale;
        self
    }

    pub fn vertex_position(mut self, position: Isometry<f32>) -> Self {
        self.vertex_offset = position;
        self
    }

    pub fn vertex_rotation(mut self, rotation: Rotation<f32>) -> Self {
        self.vertex_offset.rotation = rotation;
        self
    }

    pub fn vertex_translation(mut self, translation: Vector<f32>) -> Self {
        self.vertex_offset.translation.vector = translation;
        self
    }

    pub fn vertex_rotation_axis(mut self, rotation_axis: Vector<f32>) -> Self {
        self.vertex_rotation_axis = rotation_axis;
        self
    }

    pub fn tex_coord_scale(mut self, scale: Vector<f32>) -> Self {
        self.tex_coord_scale = scale;
        self
    }

    pub fn tex_coord_position(mut self, position: Isometry<f32>) -> Self {
        self.tex_coord_offset = position;
        self
    }

    pub fn tex_coord_rotation(mut self, rotation: Rotation<f32>) -> Self {
        self.tex_coord_offset.rotation = rotation;
        self
    }

    pub fn tex_coord_translation(mut self, translation: Vector<f32>) -> Self {
        self.tex_coord_offset.translation.vector = translation;
        self
    }

    pub fn tex_coord_rotation_axis(mut self, rotation_axis: Vector<f32>) -> Self {
        self.tex_coord_rotation_axis = rotation_axis;
        self
    }

    pub fn apply_modifiers(mut self) -> Self {
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
        vertices: &mut Vec<Vertex>,
        vertex_offset: Isometry<f32>,
        tex_coord_offset: Isometry<f32>,
        vertex_scale: Vector<f32>,
        tex_coord_scale: Vector<f32>,
        vertex_rotation_axis: Vector<f32>,
        tex_coord_rotation_axis: Vector<f32>,
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
                v.tex_coords.x *= tex_coord_scale.x;
                v.tex_coords.y *= tex_coord_scale.y;
            }
        }

        let angle = tex_coord_offset.rotation.angle();
        if angle != Self::DEFAULT_ROTATION {
            for v in vertices.iter_mut() {
                let delta = v.tex_coords - tex_coord_rotation_axis;
                v.tex_coords = tex_coord_rotation_axis + tex_coord_offset.rotation * delta;
            }
        }

        if tex_coord_offset.translation.vector != Self::DEFAULT_OFFSET {
            for v in vertices.iter_mut() {
                v.tex_coords += tex_coord_offset.translation.vector;
            }
        }
    }

    pub fn build(self, gpu: &Gpu) -> Model {
        let mut vertices = self.vertices.clone();
        assert!(vertices.len() >= 3);
        Self::compute_modifed_vertices(
            &mut vertices,
            self.vertex_offset,
            self.tex_coord_offset,
            self.vertex_scale,
            self.tex_coord_scale,
            self.vertex_rotation_axis,
            self.tex_coord_rotation_axis,
        );

        let vertex_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("vertex_buffer"),
                contents: bytemuck::cast_slice(&vertices[..]),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

        let index_buffer;
        if self.indices[..] == ModelBuilder::TRIANGLE_INDICES {
            index_buffer = ModelIndexBuffer::Triangle;
        } else if self.indices[..] == ModelBuilder::CUBOID_INDICES {
            index_buffer = ModelIndexBuffer::Cuboid;
        } else {
            index_buffer = ModelIndexBuffer::Custom(gpu.device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("index_buffer"),
                    contents: bytemuck::cast_slice(&self.indices[..]),
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                },
            ));
        }

        Model {
            amount_of_vertices: self.vertices.len() as u32,
            amount_of_indices: self.indices.len() as u32,
            vertex_buffer,
            index_buffer,
            aabb: AABB::from_vertices(&vertices),
        }
    }
}

/// 2D Model represented by its [Vertices](Vertex) and [Indices](Index).
#[derive(Debug)]
pub struct Model {
    amount_of_vertices: u32,
    amount_of_indices: u32,
    vertex_buffer: wgpu::Buffer,
    index_buffer: ModelIndexBuffer,
    aabb: AABB,
}

impl Model {
    pub fn new(gpu: &Gpu, builder: ModelBuilder) -> Self {
        builder.build(gpu)
    }

    pub fn intersects_camera(&self, position: Isometry<f32>, camera: &CameraBuffer) -> bool {
        let model_aabb = self.aabb(position);
        let camera_aabb = camera.model().aabb(Vector::default().into());
        camera_aabb.intersects(&model_aabb)
    }

    pub fn write(&mut self, gpu: &Gpu, builder: ModelBuilder) {
        let builder = builder.apply_modifiers();
        self.write_indices(gpu, &builder.indices);
        self.write_vertices(gpu, &builder.vertices);
    }

    pub fn write_indices(&mut self, gpu: &Gpu, indices: &[Index]) {
        assert_eq!(indices.len(), self.amount_of_indices as usize);
        match &self.index_buffer {
            ModelIndexBuffer::Custom(c) => {
                gpu.queue
                    .write_buffer(c, 0, bytemuck::cast_slice(&indices[..]));
            }
            _ => {
                self.index_buffer = ModelIndexBuffer::Custom(gpu.device.create_buffer_init(
                    &wgpu::util::BufferInitDescriptor {
                        label: Some("index_buffer"),
                        contents: bytemuck::cast_slice(&indices[..]),
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    },
                ));
            }
        };
    }

    pub fn write_vertices(&mut self, gpu: &Gpu, vertices: &[Vertex]) {
        assert_eq!(vertices.len(), self.amount_of_vertices as usize);
        self.aabb = AABB::from_vertices(vertices);
        gpu.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices[..]));
    }

    pub fn vertex_buffer(&self) -> &wgpu::Buffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &ModelIndexBuffer {
        &self.index_buffer
    }

    pub fn amount_of_indices(&self) -> u32 {
        self.amount_of_indices * 3
    }

    pub fn amount_of_vertices(&self) -> u32 {
        self.amount_of_vertices
    }

    pub fn aabb(&self, position: Isometry<f32>) -> AABB {
        self.aabb.with_position(position)
    }
}
