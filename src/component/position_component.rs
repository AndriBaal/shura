use crate::{
    component::Component,
    graphics::{Color, Instance2D, Instance3D, SpriteAtlas, SpriteSheetIndex},
    math::{Isometry2, Isometry3, Rotation2, Rotation3, Vector2, Vector3},
    physics::World,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionComponent2D {
    scaling: Vector2<f32>,
    position: Isometry2<f32>,
    active: bool,
    instance: Instance2D,
}

impl Default for PositionComponent2D {
    fn default() -> Self {
        Self {
            scaling: Vector2::new(1.0, 1.0),
            instance: Instance2D::default(),
            position: Isometry2::default(),
            active: true,
        }
    }
}

#[allow(unreachable_patterns)]
impl PositionComponent2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.set_rotation(Rotation2::new(rotation));
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

    pub fn with_active(mut self, active: bool) -> Self {
        self.set_active(active);
        self
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn set_rotation(&mut self, rotation: Rotation2<f32>) {
        self.position.rotation = rotation;
        self.instance.set_rotation_scaling(self.scaling, rotation);
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.instance.set_translation(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.position = position;
        self.instance = Instance2D::new_position(position, self.scaling);
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn rotation(&self) -> Rotation2<f32> {
        self.position.rotation
    }

    pub fn translation(&self) -> Vector2<f32> {
        self.position.translation.vector
    }

    pub fn position(&self) -> Isometry2<f32> {
        self.position
    }

    pub const fn scaling(&self) -> &Vector2<f32> {
        &self.scaling
    }

    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.scaling = scaling;
        self.instance
            .set_rotation_scaling(self.scaling, self.position.rotation);
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

    pub const fn index(&self) -> &SpriteSheetIndex {
        &self.instance.sprite_sheet_index
    }

    pub fn set_index(&mut self, index: SpriteSheetIndex) {
        self.instance.sprite_sheet_index = index;
    }

    pub fn with_index(mut self, index: SpriteSheetIndex) -> Self {
        self.set_index(index);
        self
    }
}

impl Component for PositionComponent2D {
    type Instance = Instance2D;

    fn instance(&self, _world: &World) -> Self::Instance {
        self.instance
    }

    fn active(&self) -> bool {
        self.active
    }
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
}

impl Component for PositionComponent3D {
    type Instance = Instance3D;

    fn instance(&self, _world: &World) -> Self::Instance {
        Instance3D::new(self.position, self.scaling)
    }

    fn active(&self) -> bool {
        self.active
    }
}
