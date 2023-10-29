use crate::Vector2;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct World {
    pub gravity: Vector2<f32>,
}

impl World {
    pub fn new() -> Self {
        Self {
            gravity: Default::default(),
        }
    }

    pub fn gravity(&self) -> Vector2<f32> {
        self.gravity
    }

    pub fn set_gravity(&mut self, gravity: Vector2<f32>) {
        self.gravity = gravity;
    }
}
