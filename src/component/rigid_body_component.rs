use crate::{
    component::{Component, ComponentIdentifier},
    entity::{ConstIdentifier, EntityHandle},
    math::Isometry2,
    physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle, World},
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
    pub fn get<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return world.rigid_body(*rigid_body_handle).unwrap();
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => rigid_body,
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return world.rigid_body_mut(*rigid_body_handle).unwrap();
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => rigid_body,
        }
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return world.attach_collider(*rigid_body_handle, collider)
            }
            RigidBodyComponentStatus::Uninitialized { colliders, .. } => {
                colliders.push(collider.into())
            }
        }
        None
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        match self {
            RigidBodyComponentStatus::Initialized { .. } => return world.detach_collider(collider),
            RigidBodyComponentStatus::Uninitialized { .. } => (),
        }
        None
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    pub fn position(&self, world: &World) -> Isometry2<f32> {
        *self.get(world).position()
    }

    pub fn get<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        self.status.get(world)
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        self.status.get_mut(world)
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        self.status.attach_collider(world, collider)
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        self.status.detach_collider(world, collider)
    }
}

impl ConstIdentifier for RigidBodyComponent {
    const TYPE_NAME: &'static str = "__shura_rigid_body_component";
}
impl ComponentIdentifier for RigidBodyComponent {}
impl Component for RigidBodyComponent {
    fn init(&mut self, handle: EntityHandle, world: &mut World) {
        match self.status {
            RigidBodyComponentStatus::Initialized { .. } => {}
            RigidBodyComponentStatus::Uninitialized {
                ref rigid_body,
                ref colliders,
            } => {
                let rigid_body: &RigidBody = rigid_body;
                let rigid_body_handle =
                    world.add_rigid_body(rigid_body.clone(), colliders.clone(), &handle);
                self.status = RigidBodyComponentStatus::Initialized { rigid_body_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                if let Some((rigid_body, colliders)) = world.remove_rigid_body(rigid_body_handle) {
                    self.status = RigidBodyComponentStatus::Uninitialized {
                        rigid_body: Box::new(rigid_body),
                        colliders,
                    }
                }
            }
            RigidBodyComponentStatus::Uninitialized { .. } => (),
        }
    }
}
