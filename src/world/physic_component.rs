use crate::{
    physics::{ColliderHandle, World},
    BaseComponent, ComponentHandle, Matrix,
};
use rapier2d::prelude::{ColliderBuilder, RigidBody, RigidBodyHandle};

enum BodyStatus {
    RigidBody {
        handle: RigidBodyHandle,
    },
    Pending {
        body: Box<RigidBody>,
        colliders: Vec<ColliderBuilder>,
    },
}

pub struct PhysicsComponent {
    handle: ComponentHandle,
    body: BodyStatus,
}

impl PhysicsComponent {
    pub fn new(body: impl Into<RigidBody>, colliders: Vec<ColliderBuilder>) -> Self {
        Self {
            handle: Default::default(),
            body: BodyStatus::Pending {
                body: Box::new(body.into()),
                colliders,
            },
        }
    }
}

impl PhysicsComponent {
    pub fn body<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        return match &self.body {
            BodyStatus::RigidBody { handle } => world.rigid_body(*handle),
            BodyStatus::Pending { body, .. } => body,
        };
    }

    pub fn body_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        return match &mut self.body {
            BodyStatus::RigidBody { handle } => world.rigid_body_mut(*handle).unwrap(),
            BodyStatus::Pending { body, .. } => body,
        };
    }

    pub fn collider_handles<'a>(&'a self, world: &'a World) -> &'a [ColliderHandle] {
        self.body(world).colliders()
    }

    pub fn body_handle(&self) -> Option<RigidBodyHandle> {
        return match self.body {
            BodyStatus::RigidBody { handle, .. } => Some(handle),
            _ => None,
        };
    }

    pub(crate) fn remove_from_world(&mut self, world: &mut World) {
        match std::mem::replace(
            &mut self.body,
            BodyStatus::RigidBody {
                handle: Default::default(),
            },
        ) {
            BodyStatus::Pending { body, colliders } => {
                self.body = BodyStatus::Pending { body, colliders }
            }
            BodyStatus::RigidBody { handle } => {
                world.remove_body(handle);
            },
        };
    }

    pub(crate) fn add_to_world(&mut self, world: &mut World) {
        match std::mem::replace(
            &mut self.body,
            BodyStatus::RigidBody {
                handle: Default::default(),
            },
        ) {
            BodyStatus::Pending { body, colliders } => {
                self.body = BodyStatus::RigidBody {
                    handle: world.create_body(*body),
                };
                for collider in colliders {
                    world.create_collider(self, &collider);
                }
            }
            BodyStatus::RigidBody { handle } => self.body = BodyStatus::RigidBody { handle },
        };
    }
}

impl BaseComponent for PhysicsComponent {
    fn matrix(&self, world: &World) -> Matrix {
        let body = self.body(world);
        return Matrix::new(*body.position());
    }

    fn handle(&self) -> &ComponentHandle {
        if self.handle.id() == ComponentHandle::UNINITIALIZED_ID {
            panic!("Cannot get the handle from an unadded component!");
        }
        return &self.handle;
    }

    fn init(&mut self, world: &mut World, handle: ComponentHandle) {
        if self.handle.id() == 0 {
            self.handle = handle;
            self.add_to_world(world);
        }
    }
}
