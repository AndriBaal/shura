use crate::{
    physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle},
    ComponentHandle, InstancePosition, Position, Vector, World,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RigidBodyStatus {
    Added {
        rigid_body_handle: RigidBodyHandle,
    },
    Pending {
        rigid_body: RigidBody,
        colliders: Vec<Collider>,
    },
}

impl RigidBodyStatus {
    pub fn get<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        match self {
            RigidBodyStatus::Added { rigid_body_handle } => {
                return world.rigid_body(*rigid_body_handle).unwrap();
            }
            RigidBodyStatus::Pending { rigid_body, .. } => {
                return rigid_body;
            }
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        match self {
            RigidBodyStatus::Added { rigid_body_handle } => {
                return world.rigid_body_mut(*rigid_body_handle).unwrap();
            }
            RigidBodyStatus::Pending { rigid_body, .. } => {
                return rigid_body;
            }
        }
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        match self {
            RigidBodyStatus::Added { rigid_body_handle } => {
                return world.attach_collider(*rigid_body_handle, collider)
            }
            RigidBodyStatus::Pending { colliders, .. } => colliders.push(collider.into()),
        }
        return None;
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        match self {
            RigidBodyStatus::Added { .. } => return world.detach_collider(collider),
            RigidBodyStatus::Pending { .. } => (),
        }
        return None;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RigidBodyComponent {
    pub(crate) status: RigidBodyStatus,
    pub scale: Vector<f32>,
}

impl RigidBodyComponent {
    pub fn new(
        rigid_body: impl Into<RigidBody>,
        colliders: impl IntoIterator<Item = impl Into<Collider>>,
    ) -> Self {
        Self {
            status: RigidBodyStatus::Pending {
                rigid_body: rigid_body.into(),
                colliders: colliders.into_iter().map(|c| c.into()).collect(),
            },
            scale: Vector::new(1.0, 1.0),
        }
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

    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
    }

    pub const fn scale(&self) -> &Vector<f32> {
        &self.scale
    }

    pub fn with_scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }
}

impl Position for RigidBodyComponent {
    fn instance(&self, world: &World) -> InstancePosition {
        match &self.status {
            RigidBodyStatus::Added { rigid_body_handle } => {
                if let Some(rigid_body) = world.rigid_body(*rigid_body_handle) {
                    return InstancePosition::new(
                        *rigid_body.position(),
                        if rigid_body.is_enabled() {
                            self.scale
                        } else {
                            Vector::default()
                        },
                    );
                }
            }
            RigidBodyStatus::Pending { rigid_body, .. } => {
                return InstancePosition::new(
                    *rigid_body.position(),
                    if rigid_body.is_enabled() {
                        self.scale
                    } else {
                        Vector::default()
                    },
                );
            }
        }
        return InstancePosition::default();
    }

    fn init(&mut self, handle: ComponentHandle, world: &mut World) {
        match self.status {
            RigidBodyStatus::Added { .. } => {
                return;
            }
            RigidBodyStatus::Pending {
                ref rigid_body,
                ref colliders,
            } => {
                let rigid_body_handle =
                    world.add_rigid_body(rigid_body.clone(), colliders.clone(), handle);
                self.status = RigidBodyStatus::Added { rigid_body_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            RigidBodyStatus::Added { rigid_body_handle } => {
                if let Some((rigid_body, colliders)) = world.remove_rigid_body(rigid_body_handle) {
                    self.status = RigidBodyStatus::Pending {
                        rigid_body,
                        colliders,
                    }
                }
            }
            RigidBodyStatus::Pending { .. } => return,
        }
    }
}
