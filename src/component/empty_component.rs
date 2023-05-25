use crate::{BaseComponent, Matrix};

#[cfg(feature="physics")]
use crate::physics::World;

pub struct EmptyComponent;

impl BaseComponent for EmptyComponent {
    fn matrix(&self, #[cfg(feature="physics")] world: &World) -> Matrix {
        Matrix::default()
    }
}
