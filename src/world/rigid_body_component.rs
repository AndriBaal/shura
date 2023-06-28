use crate::{
    physics::{Collider, RigidBody, RigidBodyHandle, World},
    BaseComponent, InstanceData, Vector,
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

    // pub fn attach_collider(
    //     &mut self,
    //     world: &mut World,
    //     collider: impl Into<Collider>,
    // ) -> ColliderHandle {
    //     if world.rigid_body(self.rigid_body_handle).is_some() {
    //         return world.attach_collider(self.rigid_body_handle, collider);
    //     }
    //     panic!("This RigidBodyComponent is not initailized")
    // }

    // pub fn remove_attached_colliders(
    //     &mut self,
    //     world: &mut World,
    //     rigid_body_handle: ColliderHandle,
    // ) {
    //     if let Some(collider) = world.collider(rigid_body_handle) {
    //         assert!(collider.parent().unwrap() == self.rigid_body_handle);
    //         world.unregister_collider(rigid_body_handle)
    //     }
    // }

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

impl BaseComponent for RigidBodyComponent {
    fn instance(&self, world: &World) -> crate::InstanceData {
        match &self.status {
            RigidBodyStatus::Added { rigid_body_handle } => {
                if let Some(rigid_body) = world.rigid_body(*rigid_body_handle) {
                    return InstanceData::new(
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
                return InstanceData::new(
                    *rigid_body.position(),
                    if rigid_body.is_enabled() {
                        self.scale
                    } else {
                        Vector::default()
                    },
                );
            }
        }
        return InstanceData::default();
    }
}
