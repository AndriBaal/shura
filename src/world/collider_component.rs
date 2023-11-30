use crate::{
    physics::{Collider, ColliderHandle},
    Color, Component, EntityHandle, Instance2D, SpriteAtlas, SpriteSheetIndex, Vector2, World,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum ColliderStatus {
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
                collider
            }
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        match self {
            ColliderStatus::Added { collider_handle } => {
                return world.collider_mut(*collider_handle).unwrap();
            }
            ColliderStatus::Pending { collider } => {
                collider
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub(crate) status: ColliderStatus,
    scale: Vector2<f32>,
    atlas: SpriteAtlas,
    color: Color,
    index: SpriteSheetIndex,
    active: bool,
}

impl ColliderComponent {
    pub fn new(collider: impl Into<Collider>) -> Self {
        Self {
            status: ColliderStatus::Pending {
                collider: collider.into(),
            },
            scale: Vector2::new(1.0, 1.0),
            atlas: Default::default(),
            color: Color::WHITE,
            index: 0,
            active: true,
        }
    }

    pub fn get<'a>(&'a self, world: &'a World) -> &'a Collider {
        self.status.get(world)
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        self.status.get_mut(world)
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

impl Component for ColliderComponent {
    type Instance = Instance2D;

    fn instance(&self, world: &World) -> Self::Instance {
        match &self.status {
            ColliderStatus::Added { collider_handle } => {
                if let Some(collider) = world.collider(*collider_handle) {
                    return Instance2D::new(
                        *collider.position(),
                        self.scale,
                        self.atlas,
                        self.color,
                        self.index,
                    );
                }
            }
            ColliderStatus::Pending { collider } => {
                return Instance2D::new(
                    *collider.position(),
                    self.scale,
                    self.atlas,
                    self.color,
                    self.index,
                );
            }
        }
        Instance2D::default()
    }

    fn init(&mut self, handle: EntityHandle, world: &mut World) {
        match self.status {
            ColliderStatus::Added { .. } => {
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
            ColliderStatus::Pending { .. } => (),
        }
    }

    fn active(&self) -> bool {
        self.active
    }
}
