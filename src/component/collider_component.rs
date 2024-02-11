use crate::{
    component::ComponentInstance,
    entity::EntityHandle,
    graphics::{Color, Instance2D, SpriteAtlas, SpriteSheetIndex},
    math::Vector2,
    physics::{Collider, ColliderHandle, World},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ColliderComponentStatus {
    Initialized { collider_handle: ColliderHandle },
    Uninitialized { collider: Collider },
}

impl ColliderComponentStatus {
    pub fn get<'a>(&'a self, world: &'a World) -> &'a Collider {
        match self {
            ColliderComponentStatus::Initialized { collider_handle } => {
                return world.collider(*collider_handle).unwrap();
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut Collider {
        match self {
            ColliderComponentStatus::Initialized { collider_handle } => {
                return world.collider_mut(*collider_handle).unwrap();
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub status: ColliderComponentStatus,
    scale: Vector2<f32>,
    atlas: SpriteAtlas,
    color: Color,
    index: SpriteSheetIndex,
    active: bool,
}

impl ColliderComponent {
    pub fn new(collider: impl Into<Collider>) -> Self {
        Self {
            status: ColliderComponentStatus::Uninitialized {
                collider: collider.into(),
            },
            scale: Vector2::new(1.0, 1.0),
            atlas: Default::default(),
            color: Color::WHITE,
            index: 0,
            active: true,
        }
    }

    pub fn handle(&self) -> Option<ColliderHandle> {
        match &self.status {
            ColliderComponentStatus::Initialized { collider_handle } => Some(*collider_handle),
            ColliderComponentStatus::Uninitialized { .. } => None,
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

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

impl ComponentInstance for ColliderComponent {
    type Instance = Instance2D;

    fn instance(&self, world: &World) -> Self::Instance {
        match &self.status {
            ColliderComponentStatus::Initialized { collider_handle } => {
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
            ColliderComponentStatus::Uninitialized { collider } => {
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
            ColliderComponentStatus::Initialized { .. } => {}
            ColliderComponentStatus::Uninitialized { ref collider } => {
                let collider_handle = world.add_collider(handle, collider.clone());
                self.status = ColliderComponentStatus::Initialized { collider_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            ColliderComponentStatus::Initialized { collider_handle } => {
                if let Some(collider) = world.remove_collider(collider_handle) {
                    self.status = ColliderComponentStatus::Uninitialized { collider }
                }
            }
            ColliderComponentStatus::Uninitialized { .. } => (),
        }
    }

    fn active(&self) -> bool {
        self.active
    }
}
