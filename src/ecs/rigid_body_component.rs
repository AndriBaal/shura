use crate::{
    ecs::Component,
    math::Isometry2,
    physics::{Collider, ColliderHandle, Physics, RigidBody, RigidBodyHandle},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RigidBodyComponentStatus {
    Initialized {
        rigid_body_handle: RigidBodyHandle,
    },
    Uninitialized {
        rigid_body: Box<RigidBody>,
        colliders: Vec<Collider>,
    },
}

impl RigidBodyComponentStatus {
    pub fn get<'a>(&'a self, physics: &'a Physics) -> &'a RigidBody {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return physics.rigid_body(*rigid_body_handle).unwrap();
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => rigid_body,
        }
    }

    pub fn get_mut<'a>(&'a mut self, physics: &'a mut Physics) -> &'a mut RigidBody {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return physics.rigid_body_mut(*rigid_body_handle).unwrap();
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => rigid_body,
        }
    }

    pub fn attach_collider(
        &mut self,
        physics: &mut Physics,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return physics.attach_collider(*rigid_body_handle, collider)
            }
            RigidBodyComponentStatus::Uninitialized { colliders, .. } => {
                colliders.push(collider.into())
            }
        }
        None
    }

    pub fn detach_collider(
        &mut self,
        physics: &mut Physics,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        match self {
            RigidBodyComponentStatus::Initialized { .. } => {
                return physics.detach_collider(collider)
            }
            RigidBodyComponentStatus::Uninitialized { .. } => (),
        }
        None
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Component)]
#[track(Insertion, Deletion, Removal)]
pub struct RigidBodyComponent {
    pub status: RigidBodyComponentStatus,
}

impl RigidBodyComponent {
    pub fn new(
        rigid_body: impl Into<RigidBody>,
        colliders: impl IntoIterator<Item = impl Into<Collider>>,
    ) -> Self {
        Self {
            status: RigidBodyComponentStatus::Uninitialized {
                rigid_body: Box::new(rigid_body.into()),
                colliders: colliders.into_iter().map(|c| c.into()).collect(),
            },
        }
    }

    pub fn handle(&self) -> Option<RigidBodyHandle> {
        match &self.status {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => Some(*rigid_body_handle),
            RigidBodyComponentStatus::Uninitialized { .. } => None,
        }
    }

    pub fn position(&self, physics: &Physics) -> Isometry2<f32> {
        *self.get(physics).position()
    }

    pub fn get<'a>(&'a self, physics: &'a Physics) -> &'a RigidBody {
        self.status.get(physics)
    }

    pub fn get_mut<'a>(&'a mut self, physics: &'a mut Physics) -> &'a mut RigidBody {
        self.status.get_mut(physics)
    }

    pub fn attach_collider(
        &mut self,
        physics: &mut Physics,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        self.status.attach_collider(physics, collider)
    }

    pub fn detach_collider(
        &mut self,
        physics: &mut Physics,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        self.status.detach_collider(physics, collider)
    }
}
