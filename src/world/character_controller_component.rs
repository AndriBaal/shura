use crate::{
    physics::{
        Collider, KinematicCharacterController, RigidBody, RigidBodyHandle, Shape, SharedShape,
        TypedShape, World,
    },
    BaseComponent, InstancePosition, Isometry, Rotation, Vector,
};

// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CharacterControllerComponent {
    pub controller: KinematicCharacterController,
    pub shape: SharedShape,
    scale: Vector<f32>,
    position: Isometry<f32>,
    instance: InstancePosition,
    disabled: bool,
}

impl CharacterControllerComponent {
    pub fn controller(&self) -> &KinematicCharacterController {
        &self.controller
    }

    pub fn controller_mut(&mut self) -> &mut KinematicCharacterController {
        &mut self.controller
    }

    pub fn set_controller(&mut self, controller: KinematicCharacterController) {
        self.controller = controller;
    }

    pub fn set_shape(&mut self, shape: impl Into<SharedShape>) {
        self.shape = shape.into()
    }

    pub fn try_shape<S: Shape>(&self) -> Option<&S> {
        self.shape.downcast_ref::<S>()
    }

    pub fn try_shape_mut<S: Shape>(&mut self) -> Option<&mut S> {
        self.shape.downcast_mut::<S>()
    }

    pub fn shape<S: Shape>(&self) -> &S {
        self.try_shape().unwrap()
    }

    pub fn shape_mut<S: Shape>(&mut self) -> &mut S {
        self.try_shape_mut().unwrap()
    }

    pub fn with_scale(mut self, scale: Vector<f32>) -> Self {
        self.set_scale(scale);
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

    pub fn with_shape(mut self, shape: impl Into<SharedShape>) -> Self {
        self.shape = shape.into();
        self
    }

    pub fn with_controller(mut self, controller: KinematicCharacterController) -> Self {
        self.controller = controller;
        self
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
        self.instance = InstancePosition::new(
            position,
            if self.disabled {
                Vector::default()
            } else {
                self.scale
            },
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

    // pub fn move_character(&mut self, world: &World) {}

    // pub fn move_character_no_apply(&self, world: &World) -> EffectiveCharacterMovement {

    // }

    // pub fn solve_collision(&self) {

    // }
}

impl BaseComponent for CharacterControllerComponent {
    fn instance(&self, world: &World) -> crate::InstancePosition {
        self.instance
    }
}
