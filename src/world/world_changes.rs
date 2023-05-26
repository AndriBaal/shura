use rapier2d::prelude::{ColliderHandle, RigidBodyHandle};

use crate::{
    physics::{ColliderComponent, RigidBodyComponent, World},
     ComponentDerive, ComponentHandle,
};

enum WorldChange {
    AddCollider {
        component_handle: ComponentHandle,
        collider_handle: ColliderHandle,
    },
    AddRigidBody {
        component_handle: ComponentHandle,
        rigid_body_handle: RigidBodyHandle,
    },
    RemoveCollider {
        collider_handle: ColliderHandle,
    },
    RemoveRigidBody {
        rigid_body_handle: RigidBodyHandle,
    },
}

pub(crate) struct WorldChanges {
    changes: Vec<WorldChange>,
}

impl WorldChanges {
    pub fn new() -> Self {
        Self {
            changes: Default::default(),
        }
    }

    pub fn apply(&mut self, world: &mut World) {
        for change in self.changes.drain(..) {
            match change {
                WorldChange::AddCollider {
                    component_handle,
                    collider_handle,
                } => world.register_collider(component_handle, collider_handle),
                WorldChange::AddRigidBody {
                    component_handle,
                    rigid_body_handle,
                } => world.register_rigid_body(component_handle, rigid_body_handle),
                WorldChange::RemoveCollider { collider_handle } => {
                    world.unregister_collider(collider_handle)
                }
                WorldChange::RemoveRigidBody { rigid_body_handle } => {
                    world.unregister_rigid_body(rigid_body_handle)
                }
            }
        }
    }

    pub fn register_add(
        &mut self,
        component_handle: ComponentHandle,
        component: &dyn ComponentDerive,
    ) {
        if let Some(component) = component.base().downcast_ref::<RigidBodyComponent>() {
            self.changes.push(WorldChange::AddRigidBody {
                component_handle,
                rigid_body_handle: component.rigid_body_handle,
            });
        } else if let Some(component) = component.base().downcast_ref::<ColliderComponent>() {
            self.changes.push(WorldChange::AddCollider {
                component_handle,
                collider_handle: component.collider_handle,
            });
        }
    }

    pub fn register_remove(&mut self, component: &dyn ComponentDerive) {
        if let Some(component) = component.base().downcast_ref::<RigidBodyComponent>() {
            self.changes.push(WorldChange::RemoveRigidBody {
                rigid_body_handle: component.rigid_body_handle,
            });
        } else if let Some(component) = component.base().downcast_ref::<ColliderComponent>() {
            self.changes.push(WorldChange::RemoveCollider {
                collider_handle: component.collider_handle,
            });
        }
    }
}
