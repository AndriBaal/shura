use crate::{
    ecs::Component,
    math::Isometry2,
    physics::{Collider, ColliderHandle, Physics},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderComponentStatus {
    Initialized { collider_handle: ColliderHandle },
    Uninitialized { collider: Collider },
}

impl ColliderComponentStatus {
    pub fn get<'a>(&'a self, physics: &'a Physics) -> &'a Collider {
        match self {
            ColliderComponentStatus::Initialized { collider_handle } => {
                return physics.collider(*collider_handle).unwrap();
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        }
    }

    pub fn get_mut<'a>(&'a mut self, physics: &'a mut Physics) -> &'a mut Collider {
        match self {
            ColliderComponentStatus::Initialized { collider_handle } => {
                return physics.collider_mut(*collider_handle).unwrap();
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Component)]
#[track(Insertion, Deletion, Removal)]
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

    pub fn position(&self, physics: &Physics) -> Isometry2<f32> {
        *self.get(physics).position()
    }

    pub fn get<'a>(&'a self, physics: &'a Physics) -> &'a Collider {
        self.status.get(physics)
    }

    pub fn get_mut<'a>(&'a mut self, physics: &'a mut Physics) -> &'a mut Collider {
        self.status.get_mut(physics)
    }
}
