use crate::{
    graphics::Vertex2D,
    math::{Isometry2, Vector2},
};

#[derive(Debug, Copy, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AABB {
    min: Vector2<f32>,
    max: Vector2<f32>,
}

impl AABB {
    pub fn new(p1: Vector2<f32>, p2: Vector2<f32>) -> Self {
        let min = Vector2::new(p1.x.min(p2.x), p1.y.min(p2.y));
        let max = Vector2::new(p1.x.max(p2.x), p1.y.max(p2.y));
        Self { min, max }
    }

    pub fn scale(&mut self, scale: f32) {
        self.min *= scale;
        self.max *= scale;
    }

    pub fn min(&self) -> &Vector2<f32> {
        &self.min
    }

    pub fn max(&self) -> &Vector2<f32> {
        &self.max
    }

    pub fn center(&self) -> Vector2<f32> {
        (self.min + self.max) / 2.0
    }

    pub fn half_extents(&self) -> Vector2<f32> {
        (self.max - self.min) / 2.0
    }

    pub fn dim(&self) -> Vector2<f32> {
        self.max - self.min
    }

    pub fn from_center(center: Vector2<f32>, half_extents: Vector2<f32>) -> Self {
        let min = center - half_extents;
        let max = center + half_extents;
        Self::new(min, max)
    }

    pub fn from_vertices(vertices: &[Vertex2D]) -> Self {
        if vertices.is_empty() {
            return Default::default();
        }

        let mut min_x = vertices[0].pos.x;
        let mut max_x = vertices[0].pos.x;
        let mut min_y = vertices[0].pos.y;
        let mut max_y = vertices[0].pos.y;
        for v in vertices.iter().skip(1) {
            if v.pos.x < min_x {
                min_x = v.pos.x;
            }
            if v.pos.x > max_x {
                max_x = v.pos.x;
            }

            if v.pos.y < min_y {
                min_y = v.pos.y;
            }
            if v.pos.y > max_y {
                max_y = v.pos.y;
            }
        }

        Self {
            min: Vector2::new(min_x, min_y),
            max: Vector2::new(max_x, max_y),
        }
    }

    pub fn from_position(half_extents: Vector2<f32>, position: Isometry2<f32>) -> Self {
        Self {
            min: -half_extents,
            max: half_extents,
        }
        .with_position(position)
    }

    pub fn with_translation(mut self, translation: Vector2<f32>) -> Self {
        self.min += translation;
        self.max += translation;
        self
    }

    pub fn with_position(mut self, position: Isometry2<f32>) -> Self {
        self.min += position.translation.vector;
        self.max += position.translation.vector;

        if position.rotation.angle() != 0.0 {
            let delta = self.min - position.translation.vector;
            self.min = position.translation.vector + position.rotation * delta;

            let delta = self.max - position.translation.vector;
            self.max = position.translation.vector + position.rotation * delta;

            let mut xs = [self.min.x, self.min.x, self.max.x, self.max.x];
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut ys = [self.min.y, self.min.y, self.max.y, self.max.y];
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

            self.min = Vector2::new(*xs.first().unwrap(), *ys.first().unwrap());
            self.max = Vector2::new(*xs.last().unwrap(), *ys.last().unwrap());
        }

        self
    }

    pub fn contains_point(&self, point: &Vector2<f32>) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}
