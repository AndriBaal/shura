use crate::{
    physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle},
    Color, ComponentHandle, Instance2D, InstanceHandler, SpriteAtlas, SpriteSheetIndex, Vector2,
    World,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum RigidBodyStatus {
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

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        match self {
            RigidBodyStatus::Added { rigid_body_handle } => {
                return world.attach_collider(*rigid_body_handle, collider)
            }
            RigidBodyStatus::Pending { colliders, .. } => colliders.push(collider.into()),
        }
        return None;
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        match self {
            RigidBodyStatus::Added { .. } => return world.detach_collider(collider),
            RigidBodyStatus::Pending { .. } => (),
        }
        return None;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RigidBodyInstance {
    pub(crate) status: RigidBodyStatus,
    scale: Vector2<f32>,
    atlas: SpriteAtlas,
    color: Color,
    index: SpriteSheetIndex,
    active: bool,
}

impl RigidBodyInstance {
    pub fn new(
        rigid_body: impl Into<RigidBody>,
        colliders: impl IntoIterator<Item = impl Into<Collider>>,
    ) -> Self {
        Self {
            status: RigidBodyStatus::Pending {
                rigid_body: rigid_body.into(),
                colliders: colliders.into_iter().map(|c| c.into()).collect(),
            },
            scale: Vector2::new(1.0, 1.0),
            atlas: Default::default(),
            color: Color::WHITE,
            index: 0,
            active: true,
        }
    }

    pub fn get<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        self.status.get(world)
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        self.status.get_mut(world)
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        self.status.attach_collider(world, collider)
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        self.status.detach_collider(world, collider)
    }

    pub fn with_scale(mut self, scale: Vector2<f32>) -> Self {
        self.scale = scale;
        self
    }
    pub fn set_scale(&mut self, scale: Vector2<f32>) {
        self.scale = scale;
    }

    pub const fn scale(&self) -> &Vector2<f32> {
        &self.scale
    }

    pub fn with_atlas(mut self, atlas: SpriteAtlas) -> Self {
        self.atlas = atlas;
        self
    }
    pub fn set_atlas(&mut self, atlas: SpriteAtlas) {
        self.atlas = atlas;
    }

    pub const fn atlas(&self) -> &SpriteAtlas {
        &self.atlas
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub const fn color(&self) -> &Color {
        &self.color
    }

    pub fn with_index(mut self, index: SpriteSheetIndex) -> Self {
        self.index = index;
        self
    }
    pub fn set_index(&mut self, index: SpriteSheetIndex) {
        self.index = index;
    }

    pub const fn index(&self) -> &SpriteSheetIndex {
        &self.index
    }
}

impl InstanceHandler for RigidBodyInstance {
    type Instance = Instance2D;

    fn instance(&self, world: &World) -> Self::Instance {
        match &self.status {
            RigidBodyStatus::Added { rigid_body_handle } => {
                if let Some(rigid_body) = world.rigid_body(*rigid_body_handle) {
                    return Instance2D::new(
                        *rigid_body.position(),
                        self.scale,
                        self.atlas,
                        self.color,
                        self.index,
                    );
                }
            }
            RigidBodyStatus::Pending { rigid_body, .. } => {
                return Instance2D::new(
                    *rigid_body.position(),
                    self.scale,
                    self.atlas,
                    self.color,
                    self.index,
                );
            }
        }
        return Instance2D::default();
    }

    fn init(&mut self, handle: ComponentHandle, world: &mut World) {
        match self.status {
            RigidBodyStatus::Added { .. } => {
                return;
            }
            RigidBodyStatus::Pending {
                ref rigid_body,
                ref colliders,
            } => {
                let rigid_body_handle =
                    world.add_rigid_body(rigid_body.clone(), colliders.clone(), handle);
                self.status = RigidBodyStatus::Added { rigid_body_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            RigidBodyStatus::Added { rigid_body_handle } => {
                if let Some((rigid_body, colliders)) = world.remove_rigid_body(rigid_body_handle) {
                    self.status = RigidBodyStatus::Pending {
                        rigid_body,
                        colliders,
                    }
                }
            }
            RigidBodyStatus::Pending { .. } => return,
        }
    }

    fn active(&self) -> bool {
        self.active
    }
}
