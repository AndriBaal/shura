use crate::{ComponentHandle, Isometry, Matrix, Rotation, Vector};

/// Easily create a [BaseComponent] with a position and render_scale.
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

    pub fn build(self) -> BaseComponent {
        self.into()
    }
}

impl Into<BaseComponent> for PositionBuilder {
    fn into(self) -> BaseComponent {
        return BaseComponent::new(self);
    }
}

/// Base of a component that is bound to a poisition on the screen, either by a
/// Position or a [RigidBody (physics only)](crate::physics::RigidBody).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct BaseComponent {
    handle: ComponentHandle,
    render_scale: Vector<f32>,
    position: Isometry<f32>,
    matrix: Matrix,
}

impl Default for BaseComponent {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[allow(unreachable_patterns)]
impl BaseComponent {
    pub fn new(pos: PositionBuilder) -> Self {
        let mut matrix = Matrix::NULL_MODEL;
        matrix.translate(pos.position.translation.vector);
        matrix.rotate(pos.render_scale, pos.position.rotation);
        Self {
            handle: Default::default(),
            render_scale: pos.render_scale,
            matrix: matrix,
            position: pos.position,
        }
    }

    pub(crate) fn init(&mut self, handle: ComponentHandle) {
        self.handle = handle;
    }

    pub(crate) fn deinit(&mut self) {
        self.handle = ComponentHandle::INVALID;
    }

    pub fn matrix(&self) -> Matrix {
        self.matrix
    }

    pub fn handle(&self) -> ComponentHandle {
        return self.handle;
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
