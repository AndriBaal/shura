use downcast_rs::Downcast;

use crate::{ComponentBufferManager, EntityHandle, EntityIdentifier, Instance, World};

#[allow(unused_variables)]
pub trait Component: Downcast {
    type Instance: Instance;
    fn instance(&self, world: &World) -> Self::Instance;
    fn active(&self) -> bool;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}

pub trait Entity: EntityIdentifier + Sized + 'static {
    fn buffer<'a>(
        entities: impl Iterator<Item = &'a Self>,
        buffers: &mut ComponentBufferManager,
        world: &World,
    );
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}
