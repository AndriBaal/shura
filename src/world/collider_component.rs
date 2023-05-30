use crate::{
    physics::{Collider, ColliderHandle, World},
    BaseComponent, Matrix, Vector,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub collider_handle: ColliderHandle,
}

impl ColliderComponent {
    pub fn new(world: &mut World, collider: impl Into<Collider>) -> Self {
        world.create_collider_component(collider)
    }

    pub fn get<'a>(&self, world: &'a World) -> &'a Collider {
        world.collider(self.collider_handle).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Collider {
        world.collider_mut(self.collider_handle).unwrap()
    }
}

impl BaseComponent for ColliderComponent {
    fn matrix(&self, world: &World) -> Matrix {
        if let Some(collider) = world.collider(self.collider_handle) {
            return Matrix::new(*collider.position(), Vector::new(1.0, 1.0));
        }
        return Matrix::default();
    }
}
