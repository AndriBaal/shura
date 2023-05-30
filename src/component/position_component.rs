use crate::{Isometry, Matrix, Rotation, Vector, BaseComponent};

#[cfg(feature="physics")]
use crate::physics::World;

/// Easily create a [PositionComponent] with a position and render_scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionBuilder {
    pub render_scale: Vector<f32>,
    pub position: Isometry<f32>,
}

impl Default for PositionBuilder {
    fn default() -> Self {
        Self {
            render_scale: Vector::new(1.0, 1.0),
            position: Default::default(),
        }
    }
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render_scale(mut self, render_scale: Vector<f32>) -> Self {
        self.render_scale = render_scale;
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

/// Base of a component that is bound to a poisition on the screen, either by a
/// Position or a [RigidBody (physics only)](crate::physics::RigidBody).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct PositionComponent {
    render_scale: Vector<f32>,
    position: Isometry<f32>,
    matrix: Matrix,
}

impl Default for PositionComponent {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[allow(unreachable_patterns)]
impl PositionComponent {
    pub fn new(pos: PositionBuilder) -> Self {
        let mut matrix = Matrix::NULL_MODEL;
        matrix.translate(pos.position.translation.vector);
        matrix.rotate(pos.render_scale, pos.position.rotation);
        Self {
            render_scale: pos.render_scale,
            matrix: matrix,
            position: pos.position,
        }
    }

    pub fn matrix(&self) -> Matrix {
        self.matrix
    }
    
    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        self.matrix.rotate(self.render_scale, rotation);
        self.position.rotation = rotation;
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.matrix.translate(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.matrix = Matrix::new(position, self.render_scale);
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

    pub const fn render_scale(&self) -> &Vector<f32> {
        &self.render_scale
    }

    pub fn set_scale(&mut self, render_scale: Vector<f32>) {
        self.render_scale = render_scale;
        self.matrix
            .rotate(self.render_scale, self.position.rotation);
    }
}

impl BaseComponent for PositionComponent {
    fn matrix(&self, #[cfg(feature="physics")] _world: &World) -> Matrix {
        self.matrix
    }
}
