use crate::{BaseComponent, Matrix};

#[cfg(feature = "physics")]
use crate::physics::World;

#[derive(Copy, Clone, Default)]
pub struct EmptyComponent;

impl EmptyComponent {
    pub fn new() -> Self {
        Self
    }
}

impl BaseComponent for EmptyComponent {
    fn matrix(&self, #[cfg(feature = "physics")] _world: &World) -> Matrix {
        Matrix::default()
    }
}
