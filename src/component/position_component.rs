use crate::{BaseComponent, InstanceData, Isometry, Rotation, Vector};

#[cfg(feature = "physics")]
use crate::physics::World;

/// Easily create a [PositionComponent] with a position and scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionBuilder {
    pub scale: Vector<f32>,
    pub position: Isometry<f32>,
}

impl Default for PositionBuilder {
    fn default() -> Self {
        Self {
            scale: Vector::new(1.0, 1.0),
            position: Default::default(),
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
            instance: InstanceData::new(pos.position, pos.scale),
            position: pos.position,
        }
    }

    pub fn instance(&self) -> InstanceData {
        self.instance
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.instance.set_scale_rotation(self.scale, rotation);
        self.position.rotation = rotation;
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.instance.set_translation(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.instance = InstanceData::new(position, self.scale);
        self.position = position;
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

    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
        self.instance
            .set_scale_rotation(self.scale, self.position.rotation);
    }
}

impl BaseComponent for PositionComponent {
    fn instance(&self, #[cfg(feature = "physics")] _world: &World) -> InstanceData {
        self.instance
    }
}
