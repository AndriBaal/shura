use downcast_rs::{Downcast, impl_downcast};

use crate::{ComponentHandle, ComponentIdentifier, Instance, World};

#[allow(unused_variables)]
pub trait InstanceHandler: Downcast {
    type Instance: Instance
    where
        Self: Sized;
    fn instance(&self, world: &World) -> Self::Instance
    where
        Self: Sized;
    fn active(&self) -> bool;
    fn init(&mut self, handle: ComponentHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}

/// All components need to implement from this trait. This is not done manually, but with the derive macro [Component](crate::Component).
pub trait Component: ComponentIdentifier + Sized + 'static {
    type InstanceHandler: InstanceHandler;
    fn handler(&self) -> &Self::InstanceHandler;
    fn init(&mut self, handle: ComponentHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}
