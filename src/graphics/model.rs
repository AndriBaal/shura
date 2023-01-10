#[cfg(feature = "physics")]
use crate::physics::{ColliderBuilder, TypedShape};
use crate::{Dimension, Gpu, Index, Isometry, Rotation, Vector, Vertex};
use std::f32::consts::PI;
use wgpu::util::DeviceExt;

// #[cfg(feature = "physics")]
// use rapier2d::prelude::ColliderBuilder;

#[derive(Clone, Debug)]
/// Shape of a [Model].
pub enum ModelShape {
    Ball {
        radius: f32,
        resolution: u32,
    },
    Cuboid {
        half_extents: Dimension<f32>,
    },
    Capsule {
        half_height: f32,
        radius: f32,
        resolution: u32,
    },
    Triangle {
        a: Vector<f32>,
        b: Vector<f32>,
        c: Vector<f32>,
    },
    Segment {
        a: Vector<f32>,
        b: Vector<f32>,
        half_thickness: f32,
    },
    TriMesh {
        vertices: Vec<Vector<f32>>,
        indices: Option<Vec<Index>>,
    },
    ConvexPolygon {
        vertices: Vec<Vector<f32>>,
    },
    Compound {
        shapes: Vec<(Isometry<f32>, Vector<f32>, ModelShape)>,
    },
    PolyLine {
        lines: Vec<(Vector<f32>, Vector<f32>, f32)>,
    },
    RoundCuboid {
        half_extents: Dimension<f32>,
        border_radius: f32,
    },
    RoundTriangle {
        a: Vector<f32>,
        b: Vector<f32>,
        c: Vector<f32>,
        border_radius: f32,
    },
    RoundConvexPolygon {
        vertices: Vec<Vector<f32>>,
        border_radius: f32,
    },
    Custom,
}

#[cfg(feature = "physics")]
impl ModelShape {
    fn from_collider_shape(
        shape: TypedShape,
        resolution: u32,
        half_thickness: f32,
    ) -> Option<Self> {
        return match shape {
            TypedShape::Ball(ball) => Some(ModelShape::Ball {
                radius: ball.radius,
                resolution,
            }),
            TypedShape::Cuboid(cuboid) => Some(ModelShape::Cuboid {
                half_extents: cuboid.half_extents.into(),
            }),
            TypedShape::Capsule(capsule) => Some(ModelShape::Capsule {
                half_height: capsule.half_height(),
                radius: capsule.radius,
                resolution,
            }),
            TypedShape::Segment(segment) => Some(ModelShape::Segment {
                a: segment.a.coords,
                b: segment.b.coords,
                half_thickness,
            }),
            TypedShape::Triangle(triangle) => Some(ModelShape::Triangle {
                a: triangle.a.coords,
                b: triangle.b.coords,
                c: triangle.c.coords,
            }),
            TypedShape::TriMesh(tri_mesh) => Some(ModelShape::TriMesh {
                vertices: tri_mesh.vertices().iter().map(|p| p.coords).collect(),
                indices: tri_mesh
                    .indices()
                    .iter()
                    .map(|p| Some(Index::new(p[0], p[1], p[2])))
                    .collect(),
            }),
            TypedShape::Polyline(compound) => Some(ModelShape::PolyLine {
                lines: compound
                    .segments()
                    .map(|s| (s.a.coords, s.b.coords, half_thickness))
                    .collect(),
            }),

            TypedShape::Compound(compound) => {
                let mut shapes = vec![];
                for (pos, shape) in compound.shapes() {
                    if let Some(model_shape) = Self::from_collider_shape(
                        shape.as_typed_shape(),
                        resolution,
                        half_thickness,
                    ) {
                        shapes.push((*pos, Vector::new(1.0 as f32, 1.0), model_shape));
                    }
                }
                Some(ModelShape::Compound { shapes })
            }
            TypedShape::ConvexPolygon(convex_polygon) => Some(ModelShape::ConvexPolygon {
                vertices: convex_polygon.points().iter().map(|p| p.coords).collect(),
            }),
            TypedShape::RoundCuboid(round_cuboid) => Some(ModelShape::RoundCuboid {
                half_extents: round_cuboid.inner_shape.half_extents.into(),
                border_radius: round_cuboid.border_radius,
            }),
            TypedShape::RoundTriangle(round_triangle) => Some(ModelShape::RoundTriangle {
                a: round_triangle.inner_shape.a.coords,
                b: round_triangle.inner_shape.b.coords,
                c: round_triangle.inner_shape.c.coords,
                border_radius: round_triangle.border_radius,
            }),
            TypedShape::RoundConvexPolygon(round_convex_polygon) => {
                Some(ModelShape::RoundConvexPolygon {
                    vertices: round_convex_polygon
                        .inner_shape
                        .points()
                        .iter()
                        .map(|p| p.coords)
                        .collect(),
                    border_radius: round_convex_polygon.border_radius,
                })
            }
            TypedShape::HalfSpace(_) => None,
            TypedShape::HeightField(_) => None,
            TypedShape::Custom(_) => None,
        };
    }
}

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

/// Builder to easily create a [Model].
#[derive(Clone)]
pub struct ModelBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<Index>,
    pub vertex_offset: Isometry<f32>,
    pub tex_coord_offset: Isometry<f32>,
    pub vertex_scale: Vector<f32>,
    pub tex_coord_scale: Vector<f32>,
    pub shape: ModelShape,
}

struct WrapIter<'a> {
    len: usize,
    vertices: &'a Vec<Vertex>,
}

impl<'a> WrapIter<'a> {
    pub fn new(vertices: &'a Vec<Vertex>) -> WrapIter<'a> {
        Self {
            len: vertices.len() - 1,
            vertices,
        }
    }
}

impl<'a> Iterator for WrapIter<'a> {
    type Item = (usize, usize, Vertex, Vertex);
    fn next(&mut self) -> Option<Self::Item> {
        self.len += 1;
        return Some((
            self.len,
            self.len + 1,
            self.vertices[self.len],
            self.vertices[self.len + 1],
        ));
    }
}

impl ModelBuilder {
    const DEFAULT_OFFSET: Vector<f32> = Vector::new(0.0, 0.0);
    const DEFAULT_ROTATION: f32 = 0.0;
    const DEFAULT_SCALE: Vector<f32> = Vector::new(1.0, 1.0);
    fn round_vertices(vertices: Vec<Vertex>, border_radius: f32, resolution: u32) -> Vec<Vertex> {
        let pi_cos = PI.cos();
        let pi_sin = PI.sin();

        let vertices = vec![];

        // let n = vec![];
        // let a = WrapIter::new(&vertices).map(|(_, _, v0, v1)| v0.pos.angle(&v1.pos)).collect::<Vec<f32>>();
        // for (_, _, v0, v1) in WrapIter::new(&vertices) {
        //     let s = v1.pos - v0.pos;
        //     let t = Vector::new(s.x*pi_cos + s.y*-pi_sin, s.x*pi_sin + s.y * pi_cos).normalize() * border_radius;
        //     n.push(Vertex::new(t,  Vector::new(0.0, 0.0)));
        // }

        return vertices;
        //     n = [scale(normalize(mul(ccw_left, sub(v1,v0))), border) for _,_,v0,v1 in wrap(v)]
        //     a = [get_angle(n0,n1) for _,_,n0,n1 in wrap(n)]
        //     v_prime = [list(v0) for v0 in v]
        //     for i,j,k in double_wrap(v):
        //         a_prime = (math.pi - a[i])/2
        //         h = border/math.tan(a_prime)
        //         v_prime[j] = sub(add(v[j],scale(normalize(sub(v[k],v[j])),h)),n[j])
        //     v = v_prime
        //     s = [get_steps(a0) for a0 in a]
        //     for i in range(len(a)):
        //         if s[i] > 0:
        //             a[i] /= s[i]
        //             s[i] -= 1
        //     v_new = [None] * (2*len(v) + sum(s))
        //     index = 0
        //     for i,j,v0,v1 in wrap(v):
        //         v_new[index] = add(v0, n[i])
        //         index += 1
        //         v_new[index] = add(v1, n[i])
        //         index += 1
        //         m = gen_rot_mat(-a[i])
        //         step = n[i]
        //         for _ in range(s[i]):
        //             step = mul(m, step)
        //             v_new[index] = add(v1, step)
        //             index += 1
        //     return v_new
    }

    pub fn new(shape: ModelShape) -> Self {
        return match shape {
            ModelShape::Ball { radius, resolution } => Self::ball(radius, resolution),
            ModelShape::Cuboid { half_extents } => Self::cuboid(half_extents),
            // ModelShape::Capsule {
            //     half_height,
            //     radius,
            //     resolution,
            // } => Self::capsule(half_height, radius, resolution),
            // ModelShape::Triangle { a, b, c } => Self::triangle(a, b, c),
            // ModelShape::Segment {
            //     a,
            //     b,
            //     half_thickness,
            // } => Self::segment(a, b, half_thickness),
            // ModelShape::TriMesh { vertices, indices } => Self::tri_mesh(vertices, indices),
            // ModelShape::ConvexPolygon { vertices } => Self::convex_polygon(vertices),
            // ModelShape::Compound { shapes } => Self::compound(shapes),
            // ModelShape::PolyLine { lines } => Self::poly_line(lines),
            // ModelShape::RoundCuboid {
            //     half_extents,
            //     border_radius,
            // } => Self::round_cuboid(half_extents, border_radius),
            // ModelShape::RoundTriangle {
            //     a,
            //     b,
            //     c,
            //     border_radius,
            // } => Self::round_triangle(a, b, c, border_radius),
            // ModelShape::RoundConvexPolygon {
            //     vertices,
            //     border_radius,
            // } => Self::round_convex_polygon(vertices, border_radius),
            // ModelShape::Custom { vertices, indices } => Self::custom(vertices, indices),
            _ => unimplemented!(),
        };
    }

    // #[cfg(feature = "physics")]
    // pub fn from_collider_shape(
    //     shape: TypedShape,
    //     resolution: u32,
    //     half_thickness: f32,
    // ) -> Option<Self> {
    //     let shape = ModelShape::from_collider_shape(shape);
    // }

    // #[cfg(feature = "physics")]
    // pub fn into_collider(&self, shape: TypedShape, resolution: u32) -> Option<ColliderBuilder> {}

    // pub fn segment(a: Vector<f32>, b: Vector<f32>, half_thickness: f32) -> Self {
    //     let rot = a.angle(&b);
    //     let angles = Vector::new(rot.cos(), rot.sin());

    //     let mut min = Vector::new(0.0, 0.0);
    //     let mut max = Vector::new(0.0, 0.0);

    //     let mut indices = vec![Index::new(0, 1, 2), Index::new(2, 3, 0)];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::Segment {
    //             a,
    //             b,
    //             half_thickness,
    //         },
    //     }
    // }

    // pub fn round_cuboid(half_extents: Dimension<f32>, border_radius: f32) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    // vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    // vertex_scale: Self::DEFAULT_SCALE,
    // tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    // tex_coord_scale: Self::DEFAULT_SCALE,Index
    //         vertices,
    //         indices,
    //         shape: ModelShape::RoundCuboid {
    //             half_extents,
    //             border_radius,
    //         },
    //     }
    // }

    // pub fn round_triangle(
    //     a: Vector<f32>,
    //     b: Vector<f32>,
    //     c: Vector<f32>,
    //     border_radius: f32,
    // ) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::RoundTriangle {
    //             a,
    //             b,
    //             c,
    //             border_radius,
    //         },
    //     }
    // }

    // pub fn round_convex_polygon(vertices: Vec<Vector<f32>>, border_radius: f32) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::RoundConvexPolygon {
    //             vertices,
    //             border_radius,
    //         },
    //     }
    // }

    // pub fn convex_polygon(vertices: Vec<Vector<f32>>) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::ConvexPolygon { vertices },
    //     }
    // }

    // pub fn poly_line(lines: Vec<(Vector<f32>, Vector<f32>, f32)>) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::PolyLine { lines },
    //     }
    // }

    // pub fn compound(shapes: Vec<(Isometry<f32>, Vector<f32>, ModelShape)>) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::Compound { shapes },
    //     }
    // }

    // pub fn capsule(half_height: f32, radius: f32, resolution: u32) -> Self {
    //     // let mut vertices = vec![];
    //     // let mut indices = vec![];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::Capsule {
    //             half_height,
    //             radius,
    //             resolution,
    //         },
    //     }
    // }

    // pub fn triangle(a: Vector<f32>, b: Vector<f32>, c: Vector<f32>) -> Self {
    //     let mut vertices = vec![
    //         Vertex::new(a, tex_coords),
    //         Vertex::new(b, tex_coords),
    //         Vertex::new(c, tex_coords),
    //     ];
    //     let mut indices = vec![Index::new(0, 1, 2)];
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::Triangle { a, b, c },
    //     }
    // }

    // pub fn tri_mesh(vertices: Vec<Vector<f32>>, indices: Option<Vec<Index>>) -> Self {
    //     assert!(vertices.len() % 3 == 0);
    //     // let mut vertices = vec![];
    //     // for v in vertices {
    //     //     vertices.push(Vertex::new(v, tex_coords))
    //     // }
    //     // let mut indices = vec![];
    //     // for i in 0..vertices.len() {
    //     //     let index = i as u32 * 3;
    //     //     indices.push(Index::new(index, index + 1, index + 2));
    //     // }
    //     Self {
    //         vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         vertex_scale: Self::DEFAULT_SCALE,
    //         tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
    //         tex_coord_scale: Self::DEFAULT_SCALE,
    //         vertices,
    //         indices,
    //         shape: ModelShape::TriMesh { vertices, indices },
    //     }
    // }

    /// Cretae a by its half-extents [Dimension].
    pub fn cuboid(half_extents: Dimension<f32>) -> Self {
        Self {
            vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            vertex_scale: Self::DEFAULT_SCALE,
            tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            tex_coord_scale: Self::DEFAULT_SCALE,
            vertices: vec![
                Vertex::new(
                    Vector::new(-half_extents.width, half_extents.height),
                    Vector::new(0.0, 0.0),
                ),
                Vertex::new(
                    Vector::new(-half_extents.width, -half_extents.height),
                    Vector::new(0.0, 1.0),
                ),
                Vertex::new(
                    Vector::new(half_extents.width, -half_extents.height),
                    Vector::new(1.0, 1.0),
                ),
                Vertex::new(
                    Vector::new(half_extents.width, half_extents.height),
                    Vector::new(1.0, 0.0),
                ),
            ],
            indices: vec![Index::new(0, 1, 2), Index::new(2, 3, 0)],
            shape: ModelShape::Cuboid { half_extents },
        }
    }

    pub fn ball(radius: f32, resolution: u32) -> Self {
        const MIN_POINTS: u32 = 3;
        assert!(
            resolution >= MIN_POINTS,
            "A Ball must have at least {} points!",
            MIN_POINTS
        );
        let mut vertices = vec![Vertex::new(Vector::new(0.0, 0.0), Vector::new(0.5, 0.5))];
        let mut indices = vec![];
        for i in 1..resolution + 1 {
            let i = i as f32;
            let pos = Vector::new(
                radius * (i / resolution as f32 * 2.0 * PI).cos(),
                radius * (i / resolution as f32 * 2.0 * PI).sin(),
            );

            vertices.push(Vertex {
                pos,
                tex_coords: Vector::new(
                    (i / resolution as f32 * 2.0 * PI).cos() / 2.0 + 0.5,
                    (i / resolution as f32 * 2.0 * PI).sin() / -2.0 + 0.5,
                ),
            });
        }

        for i in 0..resolution {
            indices.push(Index::new(0, i, i + 1));
        }
        indices.push(Index::new(0, resolution, 1));

        Self {
            vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            vertex_scale: Self::DEFAULT_SCALE,
            tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            tex_coord_scale: Self::DEFAULT_SCALE,
            vertices,
            indices,
            shape: ModelShape::Ball { radius, resolution },
        }
    }

    pub fn custom(vertices: Vec<Vertex>, indices: Vec<Index>) -> Self {
        Self {
            vertex_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            vertex_scale: Self::DEFAULT_SCALE,
            tex_coord_offset: Isometry::new(Self::DEFAULT_OFFSET, Self::DEFAULT_ROTATION),
            tex_coord_scale: Self::DEFAULT_SCALE,
            vertices,
            indices,
            shape: ModelShape::Custom,
        }
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

    pub fn apply_modifiers(&mut self) {
        Self::compute_modifed_vertices(
            &mut self.vertices,
            self.vertex_offset,
            self.tex_coord_offset,
            self.vertex_scale,
            self.tex_coord_scale,
        )
    }

    pub fn compute_modifed_vertices(
        vertices: &mut Vec<Vertex>,
        vertex_offset: Isometry<f32>,
        tex_coord_offset: Isometry<f32>,
        vertex_scale: Vector<f32>,
        tex_coord_scale: Vector<f32>,
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
                v.pos = rotate_point_around_origin(
                    Vector::new(0.0, 0.0),
                    v.pos,
                    vertex_offset.rotation,
                );
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
                v.tex_coords = rotate_point_around_origin(
                    Vector::new(0.0, 0.0),
                    v.tex_coords,
                    tex_coord_offset.rotation,
                );
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
        Self::compute_modifed_vertices(
            &mut vertices,
            self.vertex_offset,
            self.tex_coord_offset,
            self.vertex_scale,
            self.tex_coord_scale,
        );

        let vertex_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices[..]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index_buffer"),
            contents: bytemuck::cast_slice(&self.indices[..]),
            usage: wgpu::BufferUsages::INDEX,
        });

        Model {
            amount_of_vertices: self.vertices.len() as u32,
            amount_of_indices: self.indices.len() as u32,
            vertex_buffer,
            index_buffer,
            shape: self.shape.clone(),
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

    pub fn shape(&self) -> &ModelShape {
        &self.shape
    }
}
