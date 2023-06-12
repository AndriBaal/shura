use crate::{Isometry, Vector, Vertex};

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Simple Axis-aligned minimum bounding box collision detection
pub struct AABB {
    pub min: Vector<f32>,
    pub max: Vector<f32>,
}

impl AABB {
    pub fn new(min: Vector<f32>, max: Vector<f32>) -> Self {
        Self { min, max }
    }

    pub fn center(&self) -> Vector<f32> {
        (self.min + self.max) / 2.0
    }

    pub fn half_extents(&self) -> Vector<f32> {
        (self.max - self.min) / 2.0
    }

    pub fn dim(&self) -> Vector<f32> {
        self.max - self.min
    }

    pub fn from_center(center: Vector<f32>, half_extents: Vector<f32>) -> Self {
        let min = center - half_extents;
        let max = center + half_extents;
        Self::new(min, max)
    }

    pub fn from_vertices(vertices: &[Vertex]) -> Self {
        let mut min_x = vertices[0].pos.x;
        let mut max_x = vertices[0].pos.x;
        let mut min_y = vertices[0].pos.y;
        let mut max_y = vertices[0].pos.y;
        for i in 1..vertices.len() {
            let v = vertices[i];
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

        return Self {
            min: Vector::new(min_x, min_y),
            max: Vector::new(max_x, max_y),
        };
    }

    pub fn rotated(&self, position: Isometry<f32>) -> Self {
        let mut model_aabb = self.clone();
        model_aabb.min += position.translation.vector;
        model_aabb.max += position.translation.vector;

        if position.rotation.angle() != 0.0 {
            let delta = model_aabb.min - position.translation.vector;
            model_aabb.min = position.translation.vector + position.rotation * delta;

            let delta = model_aabb.max - position.translation.vector;
            model_aabb.max = position.translation.vector + position.rotation * delta;

            let mut xs = [
                model_aabb.min.x,
                model_aabb.min.x,
                model_aabb.max.x,
                model_aabb.max.x,
            ];
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mut ys = [
                model_aabb.min.y,
                model_aabb.min.y,
                model_aabb.max.y,
                model_aabb.max.y,
            ];
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());

            model_aabb.min = Vector::new(*xs.first().unwrap(), *ys.first().unwrap());
            model_aabb.max = Vector::new(*xs.last().unwrap(), *ys.last().unwrap());
        }

        return model_aabb;
    }

    pub fn contains_point(&self, point: &Vector<f32>) -> bool {
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
