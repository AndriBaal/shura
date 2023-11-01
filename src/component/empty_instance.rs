use crate::{InstanceHandler, World};

#[derive(Copy, Clone, Default)]
/// Dummy component that should not be rendered to the screen
pub struct EmptyInstance;
pub static EMPTY_DEFAULT_COMPONENT: EmptyInstance = EmptyInstance;

impl EmptyInstance {
    pub fn new() -> Self {
        Self
    }
}

impl InstanceHandler for EmptyInstance {
    type Instance = ();

    fn instance(&self, _world: &World) -> () {
        ()
    }

    fn active(&self) -> bool {
        false
    }
}
