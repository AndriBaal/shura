use crate::{
    physics::{
        Collider, KinematicCharacterController, RigidBody, RigidBodyHandle, Shape, SharedShape,
        TypedShape, Physics,
    },
    BaseComponent, Instance2D, Isometry2, Rotation, Vector2,
};

// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CharacterControllerComponent {
    pub controller: KinematicCharacterController,
    pub shape: SharedShape,
    scaling: Vector2<f32>,
    position: Isometry2<f32>,
    instance: Instance2D,
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

    pub fn with_scaling(mut self, scaling: Vector2<f32>) -> Self {
        self.set_scaling(scaling);
        self
    }

    pub fn with_rotation(mut self, rotation: Rotation<f32>) -> Self {
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
        self.instance.set_scaling_rotation(
            if disabled {
                Vector2::default()
            } else {
                self.scaling
            },
            self.position.rotation,
        );
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.position.rotation = rotation;
        self.instance.set_scaling_rotation(
            if self.disabled {
                Vector2::default()
            } else {
                self.scaling
            },
            rotation,
        );
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.instance.set_translation(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.position = position;
        self.instance = Instance2D::new(
            position,
            if self.disabled {
                Vector2::default()
            } else {
                self.scaling
            },
        );
    }

    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.scaling = scaling;
        self.instance.set_scaling_rotation(
            if self.disabled {
                Vector2::default()
            } else {
                self.scaling
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

    pub fn translation(&self) -> Vector2<f32> {
        self.position.translation.vector
    }

    pub fn position(&self) -> Isometry2<f32> {
        self.position
    }

    pub const fn scaling(&self) -> &Vector2<f32> {
        &self.scaling
    }

    // pub fn move_character(&mut self, physics: &Physics) {}

    // pub fn move_character_no_apply(&self, physics: &Physics) -> EffectiveCharacterMovement {

    // }

    // pub fn solve_collision(&self) {

    // }
}

impl BaseComponent for CharacterControllerComponent {
    fn instance(&self, physics: &Physics) -> crate::Instance2D {
        self.instance
    }
}
