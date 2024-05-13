use crate::{
    component::Component,
    entity::EntityHandle,
    graphics::{Color, Instance2D, Instance3D, RenderGroup, SpriteAtlas, SpriteArrayIndex},
    math::{Isometry2, Isometry3, Rotation3, Vector2, Vector3, AABB},
    physics::World,
};

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PositionComponent2DVisibility {
    Static(bool),
    Size(Vector2<f32>),
}

impl Default for PositionComponent2DVisibility {
    fn default() -> Self {
        Self::Static(true)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionComponent2D {
    pub visibility: PositionComponent2DVisibility,
    pub instance: Instance2D,
}

impl Default for PositionComponent2D {
    fn default() -> Self {
        Self {
            instance: Instance2D::default(),
            visibility: PositionComponent2DVisibility::default(),
            // size: None,
        }
    }
}

#[allow(unreachable_patterns)]
impl PositionComponent2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.set_rotation(rotation);
        self
    }

    pub fn with_translation(mut self, translation: Vector2<f32>) -> Self {
        self.set_translation(translation);
        self
    }

    pub fn with_position(mut self, position: Isometry2<f32>) -> Self {
        self.set_position(position);
        self
    }

    pub fn with_visibility(mut self, visibility: PositionComponent2DVisibility) -> Self {
        self.set_visibility(visibility);
        self
    }

    pub fn set_visibility(&mut self, visibility: PositionComponent2DVisibility) {
        self.visibility = visibility;
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        self.instance.rotation = rotation;
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.instance.translation = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.instance.set_position(position)
    }

    pub fn visibility(&self) -> PositionComponent2DVisibility {
        self.visibility
    }

    pub fn rotation(&self) -> f32 {
        self.instance.rotation
    }

    pub fn translation(&self) -> Vector2<f32> {
        self.instance.translation
    }

    pub fn position(&self) -> Isometry2<f32> {
        self.instance.position()
    }

    pub fn scaling(&self) -> Vector2<f32> {
        self.instance.scaling
    }

    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.instance.scaling = scaling;
    }

    pub fn with_scaling(mut self, scaling: Vector2<f32>) -> Self {
        self.set_scaling(scaling);
        self
    }

    pub const fn color(&self) -> &Color {
        &self.instance.color
    }

    pub fn set_color(&mut self, color: Color) {
        self.instance.color = color;
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.set_color(color);
        self
    }

    pub const fn atlas(&self) -> &SpriteAtlas {
        &self.instance.atlas
    }

    pub fn set_atlas(&mut self, atlas: SpriteAtlas) {
        self.instance.atlas = atlas;
    }

    pub fn with_atlas(mut self, atlas: SpriteAtlas) -> Self {
        self.set_atlas(atlas);
        self
    }

    pub const fn index(&self) -> &SpriteArrayIndex {
        &self.instance.sprite_array_index
    }

    pub fn set_index(&mut self, index: SpriteArrayIndex) {
        self.instance.sprite_array_index = index;
    }

    pub fn with_index(mut self, index: SpriteArrayIndex) -> Self {
        self.set_index(index);
        self
    }

    pub fn instance(&self) -> Instance2D {
        self.instance
    }
}

impl Component for PositionComponent2D {
    type Instance = Instance2D;

    fn buffer(&self, _world: &World, cam2d: &AABB, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        match self.visibility {
            PositionComponent2DVisibility::Static(s) => {
                if s {
                    render_group.push(self.instance())
                }
            }
            PositionComponent2DVisibility::Size(size) => {
                let aabb = AABB::from_center(self.translation(), size);
                if aabb.intersects(cam2d) {
                    render_group.push(self.instance())
                }
            }
        }
    }

    fn init(&mut self, _handle: EntityHandle, _world: &mut World) {}

    fn finish(&mut self, _world: &mut World) {}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionComponent3D {
    active: bool,
    position: Isometry3<f32>,
    scaling: Vector3<f32>,
}

impl Default for PositionComponent3D {
    fn default() -> Self {
        Self {
            scaling: Vector3::new(1.0, 1.0, 1.0),
            position: Isometry3::default(),
            active: true,
        }
    }
}

#[allow(unreachable_patterns)]
impl PositionComponent3D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rotation(mut self, rotation: Vector3<f32>) -> Self {
        self.set_rotation(Rotation3::new(rotation));
        self
    }

    pub fn with_translation(mut self, translation: Vector3<f32>) -> Self {
        self.set_translation(translation);
        self
    }

    pub fn with_position(mut self, position: Isometry3<f32>) -> Self {
        self.set_position(position);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.set_active(active);
        self
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn set_rotation(&mut self, rotation: Rotation3<f32>) {
        self.position.rotation = rotation;
    }

    pub fn set_translation(&mut self, translation: Vector3<f32>) {
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry3<f32>) {
        self.position = position;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn rotation(&self) -> Rotation3<f32> {
        self.position.rotation
    }

    pub fn translation(&self) -> Vector3<f32> {
        self.position.translation.vector
    }

    pub fn position(&self) -> Isometry3<f32> {
        self.position
    }

    pub const fn scaling(&self) -> &Vector3<f32> {
        &self.scaling
    }

    pub fn set_scaling(&mut self, scaling: Vector3<f32>) {
        self.scaling = scaling;
    }

    pub fn with_scaling(mut self, scaling: Vector3<f32>) -> Self {
        self.set_scaling(scaling);
        self
    }

    pub fn instance(&self) -> Instance3D {
        Instance3D::new(self.position, self.scaling)
    }
}

impl Component for PositionComponent3D {
    type Instance = Instance3D;

    fn buffer(&self, _world: &World, _cam2d: &AABB, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        // TODO: Implement AABB check
        if self.active {
            render_group.push(self.instance())
        }
    }

    fn init(&mut self, _handle: EntityHandle, _world: &mut World) {}

    fn finish(&mut self, _world: &mut World) {}
}
