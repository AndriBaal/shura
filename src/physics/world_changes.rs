use std::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    component::{
        ColliderComponent, ColliderComponentStatus, RigidBodyComponent, RigidBodyComponentStatus,
    },
    entity::{Entity, EntityHandle},
    physics::{RigidBodyHandle, ColliderHandle}
};

pub struct WorldChangeSender(Sender<WorldChange>);

impl WorldChangeSender {
    pub fn send<E: Entity>(&mut self, entity: &E) {
        for component in entity.components() {
            if let Some(component) = component.downcast_ref::<RigidBodyComponent>() {
                match &component.status {
                    RigidBodyComponentStatus::Initialized { rigid_body_handle } => self.0.send(WorldChange::RemoveRigidBody(*rigid_body_handle)).unwrap(),
                    RigidBodyComponentStatus::Uninitialized {
                        ..
                    } => (),
                }
            }

            if let Some(component) = component.downcast_ref::<ColliderComponent>() {}
        }
    }
}

pub(crate) enum WorldChange {
    RemoveRigidBody(RigidBodyHandle),
    RemoveCollider(ColliderHandle),
}

pub(crate) struct WorldChangeManager {
    sender: Sender<WorldChange>,
    receiver: Receiver<WorldChange>,
}

impl WorldChangeManager {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> WorldChangeSender {
        WorldChangeSender(self.sender.clone())
    }
}
