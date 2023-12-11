use crate::entity::EntityHandle;
use crate::graphics::Instance;
use crate::physics::World;
use downcast_rs::{impl_downcast, Downcast};

#[allow(unused_variables)]
pub trait Component: Downcast {
    type Instance: Instance
    where
        Self: Sized;
    fn instance(&self, world: &World) -> Self::Instance
    where
        Self: Sized;
    fn active(&self) -> bool;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}
impl_downcast!(Component);
