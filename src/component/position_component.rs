use crate::{BaseComponent, InstanceData, Isometry, Rotation, Vector};

#[cfg(feature = "physics")]
use crate::physics::World;

/// Easily create a [PositionComponent] with a position and scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionBuilder {
    pub scale: Vector<f32>,
    pub sprite: Vector<i32>,
    pub position: Isometry<f32>,
    pub disabled: bool,
}

impl Default for PositionBuilder {
    fn default() -> Self {
        Self {
            scale: Vector::new(1.0, 1.0),
            sprite: Vector::default(),
            position: Isometry::default(),
            disabled: false,
        }
    }
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn rotation(mut self, rotation: Rotation<f32>) -> Self {
        self.position.rotation = rotation;
        self
    }

    pub fn translation(mut self, translation: Vector<f32>) -> Self {
        self.position.translation.vector = translation;
        self
    }

    pub fn position(mut self, position: Isometry<f32>) -> Self {
        self.position = position;
        self
    }

    pub fn sprite(mut self, sprite: Vector<i32>) -> Self {
        self.sprite = sprite;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn build(self) -> PositionComponent {
        self.into()
    }
}

impl Into<PositionComponent> for PositionBuilder {
    fn into(self) -> PositionComponent {
        return PositionComponent::new(self);
    }
}

/// Component that is rendered to the screen by its given position and scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionComponent {
    scale: Vector<f32>,
    position: Isometry<f32>,
    instance: InstanceData,
    disabled: bool,
}

impl Default for PositionComponent {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[allow(unreachable_patterns)]
impl PositionComponent {
    pub fn new(pos: PositionBuilder) -> Self {
        Self {
            scale: pos.scale,
            instance: InstanceData::new(pos.position, pos.scale, pos.sprite),
            position: pos.position,
            disabled: pos.disabled,
        }
    }

    pub fn instance(&self) -> InstanceData {
        self.instance
    }

    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
        self.instance.set_scale_rotation(
            if disabled {
                Vector::default()
            } else {
                self.scale
            },
            self.position.rotation,
        );
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.position.rotation = rotation;
        self.instance.set_scale_rotation(
            if self.disabled {
                Vector::default()
            } else {
                self.scale
            },
            rotation,
        );
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.instance.set_translation(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.position = position;
        self.instance = InstanceData::new(
            position,
            if self.disabled {
                Vector::default()
            } else {
                self.scale
            },
            self.instance.sprite(),
        );
    }

    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
        self.instance.set_scale_rotation(
            if self.disabled {
                Vector::default()
            } else {
                self.scale
            },
            self.position.rotation,
        );
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    pub fn rotation(&self) -> Rotation<f32> {
        self.position.rotation
    }

    pub fn translation(&self) -> Vector<f32> {
        self.position.translation.vector
    }

    pub fn position(&self) -> Isometry<f32> {
        self.position
    }

    pub const fn scale(&self) -> &Vector<f32> {
        &self.scale
    }
}

impl BaseComponent for PositionComponent {
    fn instance(&self, #[cfg(feature = "physics")] _world: &World) -> InstanceData {
        self.instance
    }
}
