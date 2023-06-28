use crate::{
    physics::{Collider, ColliderHandle, World},
    BaseComponent, InstanceData, Vector,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderStatus {
    Added { collider_handle: ColliderHandle },
    Pending { collider: Collider },
}

impl ColliderStatus {
    pub fn get<'a>(&'a self, world: &'a World) -> &'a Collider {
        match self {
            ColliderStatus::Added { collider_handle } => {
                return world.collider(*collider_handle).unwrap();
            }
            ColliderStatus::Pending { collider } => {
                return collider;
            }
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        match self {
            ColliderStatus::Added { collider_handle } => {
                return world.collider_mut(*collider_handle).unwrap();
            }
            ColliderStatus::Pending { collider } => {
                return collider;
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub(crate) status: ColliderStatus,
    pub scale: Vector<f32>,
}

impl ColliderComponent {
    pub fn new(collider: impl Into<Collider>) -> Self {
        Self {
            status: ColliderStatus::Pending {
                collider: collider.into(),
            },
            scale: Vector::new(1.0, 1.0),
        }
    }

    pub fn get<'a>(&'a self, world: &'a World) -> &'a Collider {
        self.status.get(world)
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        self.status.get_mut(world)
    }

    pub fn with_scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }
    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
    }

    pub const fn scale(&self) -> &Vector<f32> {
        &self.scale
    }
}

impl BaseComponent for ColliderComponent {
    fn instance(&self, world: &World) -> InstanceData {
        match &self.status {
            ColliderStatus::Added { collider_handle } => {
                if let Some(collider) = world.collider(*collider_handle) {
                    return InstanceData::new(
                        *collider.position(),
                        if collider.is_enabled() {
                            self.scale
                        } else {
                            Vector::default()
                        },
                    );
                }
            }
            ColliderStatus::Pending { collider } => {
                return InstanceData::new(
                    *collider.position(),
                    if collider.is_enabled() {
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
