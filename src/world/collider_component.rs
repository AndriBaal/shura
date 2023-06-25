use rapier2d::prelude::ColliderBuilder;

use crate::{
    physics::{Collider, ColliderHandle, World},
    BaseComponent, InstanceData, Vector,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum ColliderStatus {
    Added {
        handle: ColliderHandle
    },
    Pending {
        collider: Collider
    }
}

impl ColliderStatus {
    pub fn get<'a>(&self, world: &'a World) -> &'a Collider {
        match self {
            ColliderStatus::Added { handle } => {
                return world.collider(*handle).unwrap();
            },
            ColliderStatus::Pending { collider } => {
                return collider;
            },
        }
    }

    pub fn get_mut<'a>(&mut self, world: &'a mut World) -> &'a mut Collider {
        match self {
            ColliderStatus::Added { handle } => {
                return world.collider_mut(*handle).unwrap();
            },
            ColliderStatus::Pending { collider } => {
                return collider;
            },
        }    }

    pub fn insert(&mut self, world: &mut World) {

    }

    pub fn remove(&mut self, world: &mut World) {

    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub collider_handle: ColliderHandle,
    pub sprite: Vector<i32>,
    pub scale: Vector<f32>,
}

impl ColliderComponent {
    pub fn new(world: &mut World, collider: impl Into<Collider>) -> Self {
        world.create_collider_component(collider)
    }

    pub fn get<'a>(&self, world: &'a World) -> &'a Collider {
        world.collider(self.collider_handle).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Collider {
        world.collider_mut(self.collider_handle).unwrap()
    }

    pub fn with_scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_sprite(mut self, sprite: Vector<i32>) -> Self {
        self.sprite = sprite;
        self
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
}

impl BaseComponent for ColliderComponent {
    fn instance(&self, world: &World) -> InstanceData {
        if let Some(collider) = world.collider(self.collider_handle) {
            return InstanceData::new(
                *collider.position(),
                if collider.is_enabled() {
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
