use crate::{BaseComponent, InstanceData, Isometry, Rotation, Vector};

#[cfg(feature = "physics")]
use crate::physics::World;

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
        Self {
            scale: Vector::new(1.0, 1.0),
            instance: InstanceData::default(),
            position: Isometry::default(),
            disabled: false,
        }
    }
}

#[allow(unreachable_patterns)]
impl PositionComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_scale(mut self, scale: Vector<f32>) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_rotation(mut self, rotation: Rotation<f32>) -> Self {
        self.set_rotation(rotation);
        self
    }

    pub fn with_translation(mut self, translation: Vector<f32>) -> Self {
        self.set_translation(translation);
        self
    }

    pub fn with_position(mut self, position: Isometry<f32>) -> Self {
        self.set_position(position);
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.set_disabled(disabled);
        self
    }

    pub fn with_instance(&self) -> InstanceData {
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
            }
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
