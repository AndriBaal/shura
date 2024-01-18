use crate::{
    component::ComponentCollection,
    entity::{EntityHandle, RenderEntityIterator},
    graphics::RenderGroupManager,
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::fmt::{Display, Formatter, Result};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityTypeId {
    id: u32,
}

impl EntityTypeId {
    pub const INVALID: Self = Self { id: 0 };
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

impl Display for EntityTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.id)
    }
}

pub trait EntityIdentifier: Entity {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: EntityTypeId;
    fn entity_type_id(&self) -> EntityTypeId;
}

impl std::hash::Hash for EntityTypeId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

pub trait Entity: 'static + Downcast {
    fn buffer<'a>(
        entites: impl RenderEntityIterator<'a, Self>,
        buffers: &mut RenderGroupManager,
        world: &World,
    ) where
        Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {
        for component_collection in self.component_collections_mut() {
            component_collection.init_all(handle, world);
        }
    }
    fn finish(&mut self, world: &mut World) {
        for component_collection in self.component_collections_mut() {
            component_collection.finish_all(world);
        }
    }
    fn named_components() -> &'static [&'static str]
    where
        Self: Sized;
    fn component_collections<'a>(
        &'a self,
    ) -> Box<dyn DoubleEndedIterator<Item = &dyn ComponentCollection> + 'a>;
    fn component_collections_mut<'a>(
        &'a mut self,
    ) -> Box<dyn DoubleEndedIterator<Item = &mut dyn ComponentCollection> + 'a>;

    fn component_collection<'a>(
        &'a self,
        name: &'static str,
    ) -> Option<Box<dyn DoubleEndedIterator<Item = &dyn ComponentCollection> + 'a>>;
    fn component_collection_mut<'a>(
        &'a mut self,
        name: &'static str,
    ) -> Option<Box<dyn DoubleEndedIterator<Item = &mut dyn ComponentCollection> + 'a>>;
}
impl_downcast!(Entity);
