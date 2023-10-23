use crate::{
    physics::{Collider, ColliderHandle},
    ComponentHandle, InstancePosition, Position, Vector, World,
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

impl Position for ColliderComponent {
    fn instance(&self, world: &World) -> InstancePosition {
        match &self.status {
            ColliderStatus::Added { collider_handle } => {
                if let Some(collider) = world.collider(*collider_handle) {
                    return InstancePosition::new_position(
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
                return InstancePosition::new_position(
                    *collider.position(),
                    if collider.is_enabled() {
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
            ColliderStatus::Added { .. } => {
                return;
            }
            ColliderStatus::Pending { ref collider } => {
                let collider_handle = world.add_collider(handle, collider.clone());
                self.status = ColliderStatus::Added { collider_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            ColliderStatus::Added { collider_handle } => {
                if let Some(collider) = world.remove_collider(collider_handle) {
                    self.status = ColliderStatus::Pending { collider }
                }
            }
            ColliderStatus::Pending { .. } => return,
        }
    }
}
