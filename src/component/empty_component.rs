#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{BaseComponent, ComponentHandle, ComponentTypeId, Matrix};

#[derive(Default, Debug)]
pub struct EmptyComponent {
    handle: ComponentHandle,
}

impl BaseComponent for EmptyComponent {
    fn init(
        &mut self,
        #[cfg(feature = "physics")] _world: &mut World,
        type_id: ComponentTypeId,
        handle: ComponentHandle,
    ) {
        if self.handle.id() == 0 {
            self.handle = handle;
        }
    }

    fn matrix(&self, #[cfg(feature = "physics")] _world: &World) -> Matrix {
        panic!("An Empty can not be buffered and therefore not be rendered!");
    }

    fn handle(&self) -> &ComponentHandle {
        if self.handle.id() == ComponentHandle::UNINITIALIZED_ID {
            panic!("Cannot get the handle from an unadded component!");
        }
        return &self.handle;
    }
}
