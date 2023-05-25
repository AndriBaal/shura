use crate::{physics::{RigidBody, RigidBodyHandle, World, Collider}, Vector, BaseComponent, Matrix};

pub struct RigidBodyComponent {
    pub handle: RigidBodyHandle,
}

impl RigidBodyComponent {
    pub fn new(world: &mut World, rigid_body: impl Into<RigidBody>, colliders: impl IntoIterator<Item = impl Into<Collider>>) -> Self {
        world.create_rigid_body_component(rigid_body, colliders)
    }

    pub fn get<'a>(&self, world: &'a World) -> &'a RigidBody {
        world.rigid_body(self.handle).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut RigidBody {
        world.rigid_body_mut(self.handle).unwrap()
    }

}

impl BaseComponent for RigidBodyComponent {
    fn matrix(&self, world: &World) -> crate::Matrix {
        if let Some(collider) = world.rigid_body(self.handle) {
            return Matrix::new(*collider.position(), Vector::new(1.0, 1.0));
        }
        return Matrix::default();
    }
}
