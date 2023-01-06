#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{BaseComponent, ComponentHandle, Dimension, Isometry, Matrix, Rotation, Vector};

/// Easily create a [PositionComponent].
pub struct PositionComponentBuilder {
    scale: Vector<f32>,
    position: Isometry<f32>,
}

impl Default for PositionComponentBuilder {
    fn default() -> Self {
        Self {
            scale: Vector::new(1.0, 1.0),
            position: Default::default(),
        }
    }
}

impl PositionComponentBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components X axis of its scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn scale_relative_width(mut self, window_size: Dimension<u32>) -> Self {
        self.scale.y = 1.0;
        self.scale.x = window_size.height as f32 / window_size.width as f32;
        self
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components Y axis of its scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn scale_relative_height(mut self, window_size: Dimension<u32>) -> Self {
        self.scale.x = 1.0;
        self.scale.y = window_size.width as f32 / window_size.height as f32;
        self
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
        let mut matrix = Matrix::default();
        matrix.translate(self.position.translation.vector);
        matrix.rotate(self.scale, self.position.rotation.angle());
        PositionComponent {
            handle: Default::default(),
            scale: self.scale,
            position: self.position,
            matrix,
        }
    }
}

impl Into<PositionComponent> for PositionComponentBuilder {
    fn into(self) -> PositionComponent {
        self.build()
    }
}

/// [BaseComponent] that only holds a position and a scale. This is very optimized for components with a
/// static position, since the matrix only updates when changing the component.
pub struct PositionComponent {
    handle: ComponentHandle,
    scale: Vector<f32>,
    position: Isometry<f32>,
    matrix: Matrix,
}

impl Default for PositionComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl PositionComponent {
    pub fn new() -> Self {
        Self {
            handle: Default::default(),
            scale: Vector::new(1.0, 1.0),
            matrix: Matrix::default(),
            position: Isometry::new(Vector::new(0.0, 0.0), 0.0),
        }
    }

    pub fn clicked(
        &self,
        offset: Vector<f32>,
        pos: Vector<f32>,
        dimension: Dimension<f32>,
    ) -> bool {
        let translation = self.translation();
        let scale = *self.scale();
        let half_dim: Vector<f32> = offset
            + Vector::new(
                dimension.width / 2.0 * scale.x,
                dimension.height / 2.0 * scale.y,
            );
        let bl = translation - half_dim;
        let tr = translation + half_dim;
        return pos.x > bl.x && pos.x < tr.x && pos.y > bl.y && pos.y < tr.y;
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        let angle = rotation.angle();
        self.matrix.rotate(self.scale, angle);
        self.position.rotation = Rotation::new(angle);
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        self.matrix.translate(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry<f32>) {
        self.matrix.translate(position.translation.vector);
        self.matrix.rotate(self.scale, position.rotation.angle());
        self.position = position;
    }

    pub fn rotation(&self) -> &Rotation<f32> {
        &self.position.rotation
    }

    pub fn translation(&self) -> &Vector<f32> {
        &self.position.translation.vector
    }

    pub fn position(&self) -> &Isometry<f32> {
        &self.position
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components X axis of its scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn scale_relative_width(&mut self, window_size: Dimension<u32>) {
        self.scale.y = 1.0;
        self.scale.x = window_size.height as f32 / window_size.width as f32;
        self.matrix.rotate(self.scale, self.rotation().angle());
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components Y axis of its scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn scale_relative_height(&mut self, window_size: Dimension<u32>) {
        self.scale.x = 1.0;
        self.scale.y = window_size.width as f32 / window_size.height as f32;
        self.matrix.rotate(self.scale, self.rotation().angle());
    }

    #[inline]
    pub const fn scale(&self) -> &Vector<f32> {
        &self.scale
    }

    // Setters
    #[inline]
    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
        self.matrix.rotate(self.scale, self.rotation().angle());
    }
}

impl BaseComponent for PositionComponent {
    fn init(&mut self, #[cfg(feature = "physics")] _world: &mut World, handle: ComponentHandle) {
        if self.handle.id() == 0 {
            self.handle = handle;
        }
    }

    fn matrix(&self, #[cfg(feature = "physics")] _world: &World) -> Matrix {
        return self.matrix;
    }

    fn handle(&self) -> &ComponentHandle {
        if self.handle.id() == ComponentHandle::UNINITIALIZED_ID {
            panic!("Cannot get the handle from an unadded component!");
        }
        return &self.handle;
    }
}
