use downcast_rs::Downcast;

use crate::{EntityHandle, EntityIdentifier, Instance, World};

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

pub trait Entity: EntityIdentifier + Sized + 'static {
    type Component: Component;
    fn component(&self) -> &Self::Component;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}
