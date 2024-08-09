use crate::{
    component::Component,
    entity::EntityHandle,
    math::Isometry2,
    physics::{Collider, ColliderHandle, World},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderComponentStatus {
    Initialized { collider_handle: ColliderHandle },
    Uninitialized { collider: Collider },
}

impl ColliderComponentStatus {
    pub fn get<'a>(&'a self, world: &'a World) -> &'a Collider {
        match self {
            ColliderComponentStatus::Initialized { collider_handle } => {
                return world.collider(*collider_handle).unwrap();
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        match self {
            ColliderComponentStatus::Initialized { collider_handle } => {
                return world.collider_mut(*collider_handle).unwrap();
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub status: ColliderComponentStatus,
}

impl ColliderComponent {
    pub fn new(collider: impl Into<Collider>) -> Self {
        Self {
            status: ColliderComponentStatus::Uninitialized {
                collider: collider.into(),
            },
        }
    }
}

impl ColliderComponent {
    pub fn handle(&self) -> Option<ColliderHandle> {
        match &self.status {
            ColliderComponentStatus::Initialized { collider_handle } => Some(*collider_handle),
            ColliderComponentStatus::Uninitialized { .. } => None,
        }
    }

    pub fn position(&self, world: &World) -> Isometry2<f32> {
        *self.get(world).position()
    }

    pub fn get<'a>(&'a self, world: &'a World) -> &'a Collider {
        self.status.get(world)
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        self.status.get_mut(world)
    }
}

impl Component for ColliderComponent {
    fn init(&mut self, handle: EntityHandle, world: &mut World) {
        match self.status {
            ColliderComponentStatus::Initialized { .. } => {}
            ColliderComponentStatus::Uninitialized { ref collider } => {
                let collider_handle = world.add_collider(&handle, collider.clone());
                self.status = ColliderComponentStatus::Initialized { collider_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            ColliderComponentStatus::Initialized { collider_handle } => {
                if let Some(collider) = world.remove_collider(collider_handle) {
                    self.status = ColliderComponentStatus::Uninitialized { collider }
                }
            }
            ColliderComponentStatus::Uninitialized { .. } => (),
        }
    }

    fn remove_from_world(&self, world: &mut World) {
        world.remove_no_maintain_collider(self)
    }
}
