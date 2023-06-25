use crate::{
    physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle, World},
    BaseComponent, InstanceData, Vector,
};

enum BodyStatus {
    Added {
        handle: RigidBodyHandle
    },
    Pending {
        
    }
}

impl BodyStatus {

    pub fn attach_collider(&mut self) {
        
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RigidBodyComponent {
    pub rigid_body_handle: RigidBodyHandle,
    pub sprite: Vector<i32>,
    pub scale: Vector<f32>,
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
        if world.rigid_body(self.rigid_body_handle).is_some() {
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

    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
    }

    pub const fn scale(&self) -> &Vector<f32> {
        &self.scale
    }

    pub fn set_sprite(&mut self, sprite: Vector<i32>) {
        self.sprite = sprite;
    }

    pub const fn sprite(&self) -> &Vector<i32> {
        &self.sprite
    }
    
    pub fn with_scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_sprite(mut self, sprite: Vector<i32>) -> Self {
        self.sprite = sprite;
        self
    }
}

impl BaseComponent for RigidBodyComponent {
    fn instance(&self, world: &World) -> crate::InstanceData {
        if let Some(rigid_body) = world.rigid_body(self.rigid_body_handle) {
            return InstanceData::new(
                *rigid_body.position(),
                if rigid_body.is_enabled() {
                    self.scale
                } else {
                    Vector::default()
                },
                self.sprite,
            );
        }
        return InstanceData::default();
    }
}
