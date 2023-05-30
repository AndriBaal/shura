use crate::{
    physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle, World},
    BaseComponent, Matrix, Vector,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RigidBodyComponent {
    pub rigid_body_handle: RigidBodyHandle,
}

impl RigidBodyComponent {
    pub fn new(
        world: &mut World,
        rigid_body: impl Into<RigidBody>,
        colliders: impl IntoIterator<Item = impl Into<Collider>>,
    ) -> Self {
        world.create_rigid_body_component(rigid_body, colliders)
    }

    pub fn get<'a>(&self, world: &'a World) -> &'a RigidBody {
        world.rigid_body(self.rigid_body_handle).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut RigidBody {
        world.rigid_body_mut(self.rigid_body_handle).unwrap()
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> ColliderHandle {
        if  world.rigid_body(self.rigid_body_handle).is_some() {
            return world.attach_collider(self.rigid_body_handle, collider);
        }
        panic!("This RigidBodyComponent is not initailized")
    }

    pub fn remove_attached_colliders(
        &mut self,
        world: &mut World,
        collider_handle: ColliderHandle,
    ) {
        if let Some(collider) = world.collider(collider_handle) {
            assert!(collider.parent().unwrap() == self.rigid_body_handle);
            world.unregister_collider(collider_handle)
        }
    }
}

impl BaseComponent for RigidBodyComponent {
    fn matrix(&self, world: &World) -> crate::Matrix {
        if let Some(collider) = world.rigid_body(self.rigid_body_handle) {
            return Matrix::new(*collider.position(), Vector::new(1.0, 1.0));
        }
        return Matrix::default();
    }
}
