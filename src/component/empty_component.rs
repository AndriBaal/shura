use crate::{Instance2D, Position, World};

#[derive(Copy, Clone, Default)]
/// Dummy component that should not be rendered to the screen
pub struct EmptyComponent;
pub static EMPTY_DEFAULT_COMPONENT: EmptyComponent = EmptyComponent;

impl EmptyComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Position for EmptyComponent {
    fn instance(&self, _world: &World) -> Instance2D {
        Instance2D::default()
    }

    fn active(&self) -> bool {
        false
    }
}
