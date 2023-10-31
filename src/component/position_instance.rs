use crate::{
    Color, Instance2D, Isometry2, InstanceHandler, Rotation2, SpriteAtlas, SpriteSheetIndex, Vector2,
    World,
};

/// Component that is rendered to the screen by its given position and scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionInstance {
    scale: Vector2<f32>,
    position: Isometry2<f32>,
    active: bool,
    instance: Instance2D,
}

impl Default for PositionInstance {
    fn default() -> Self {
        Self {
            scale: Vector2::new(1.0, 1.0),
            instance: Instance2D::default(),
            position: Isometry2::default(),
            active: true,
        }
    }
}

#[allow(unreachable_patterns)]
impl PositionInstance {
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
        self.instance.set_scale_rotation(
            if active {
                Vector2::default()
            } else {
                self.scale
            },
            self.position.rotation,
        );
    }

    pub fn set_rotation(&mut self, rotation: Rotation2<f32>) {
        self.position.rotation = rotation;
        self.instance.set_scale_rotation(self.scale, rotation);
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.instance.set_translation(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.position = position;
        self.instance = Instance2D::new_position(position, self.scale);
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

    pub const fn scale(&self) -> &Vector2<f32> {
        &self.scale
    }

    pub fn set_scale(&mut self, scale: Vector2<f32>) {
        self.scale = scale;
        self.instance
            .set_scale_rotation(self.scale, self.position.rotation);
    }

    pub fn with_scale(mut self, scale: Vector2<f32>) -> Self {
        self.set_scale(scale);
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

impl InstanceHandler for PositionInstance {
    type Instance = Instance2D;

    fn instance(&self, _world: &World) -> Self::Instance {
        self.instance
    }

    fn active(&self) -> bool {
        self.active
    }
}
