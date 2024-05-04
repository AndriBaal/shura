use crate::{
    component::{Component, PhysicsComponentVisibility},
    entity::EntityHandle,
    graphics::{Color, Instance2D, RenderGroup, SpriteAtlas, SpriteSheetIndex},
    math::{Vector2, AABB},
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
    pub scaling: Vector2<f32>,
    pub atlas: SpriteAtlas,
    pub color: Color,
    pub index: SpriteSheetIndex,
    pub visibility: PhysicsComponentVisibility,
}

impl ColliderComponent {
    pub fn new(collider: impl Into<Collider>) -> Self {
        Self {
            status: ColliderComponentStatus::Uninitialized {
                collider: collider.into(),
            },
            scaling: Vector2::new(1.0, 1.0),
            atlas: Default::default(),
            color: Color::WHITE,
            index: 0,
            visibility: PhysicsComponentVisibility::default(),
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

    pub fn with_scaling(mut self, scaling: Vector2<f32>) -> Self {
        self.scaling = scaling;
        self
    }
    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.scaling = scaling;
    }

    pub const fn scaling(&self) -> &Vector2<f32> {
        &self.scaling
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

    pub fn set_visibility(&mut self, visibility: PhysicsComponentVisibility) {
        self.visibility = visibility;
    }

    pub fn instance(&self, world: &World) -> Instance2D {
        let collider = match &self.status {
            ColliderComponentStatus::Initialized { collider_handle } => {
                world.collider(*collider_handle).unwrap()
            }
            ColliderComponentStatus::Uninitialized { collider } => collider,
        };
        Instance2D::new(
            collider.position().translation.vector,
            collider.position().rotation.angle(),
            self.scaling,
            self.atlas,
            self.color,
            self.index,
        )
    }
}

impl Component for ColliderComponent {
    type Instance = Instance2D;

    fn buffer(&self, world: &World, cam2d: &AABB, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        match self.visibility {
            PhysicsComponentVisibility::Static(s) => {
                if s {
                    render_group.push(self.instance(world))
                }
            }
            PhysicsComponentVisibility::Size(size) => {
                let collider = match &self.status {
                    ColliderComponentStatus::Initialized { collider_handle } => {
                        world.collider(*collider_handle).unwrap()
                    }
                    ColliderComponentStatus::Uninitialized { collider } => &*collider,
                };

                let aabb = AABB::from_center(*collider.translation(), size);
                if aabb.intersects(cam2d) {
                    render_group.push(Instance2D::new(
                        collider.position().translation.vector,
                        collider.position().rotation.angle(),
                        self.scaling,
                        self.atlas,
                        self.color,
                        self.index,
                    ))
                }
            }
            PhysicsComponentVisibility::ColliderSize => {
                let collider = match &self.status {
                    ColliderComponentStatus::Initialized { collider_handle } => {
                        world.collider(*collider_handle).unwrap()
                    }
                    ColliderComponentStatus::Uninitialized { collider } => &*collider,
                };
                let aabb: AABB = collider.compute_aabb().into();
                if aabb.intersects(cam2d) {
                    render_group.push(Instance2D::new(
                        collider.position().translation.vector,
                        collider.position().rotation.angle(),
                        self.scaling,
                        self.atlas,
                        self.color,
                        self.index,
                    ))
                }
            }
        }
    }

    fn init(&mut self, handle: EntityHandle, world: &mut World) {
        match self.status {
            ColliderComponentStatus::Initialized { .. } => {}
            ColliderComponentStatus::Uninitialized { ref collider } => {
                let collider_handle = world.add_collider(&handle, collider.clone());
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

    fn remove_from_world(&self, world: &mut World) {
        world.remove_no_maintain_collider(self)
    }
}
