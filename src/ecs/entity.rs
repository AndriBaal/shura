use crate::{ComponentBufferManager, EntityHandle, EntityIdentifier, Instance, World, EntityIter};


#[allow(unused_variables)]
pub trait Component {
    type Instance: Instance;
    fn instance(&self, world: &World) -> Self::Instance;
    fn active(&self) -> bool;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}

pub trait Entity: EntityIdentifier + Sized + 'static {
    fn buffer<'a>(
        entities: EntityIter<'a, Self>,
        buffers: &mut ComponentBufferManager,
        world: &World,
    );
    // fn components(&self) -> Vec<&dyn std::any::Any>;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}
