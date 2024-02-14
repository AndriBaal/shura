use crate::{
    component::Component,
    entity::{EntityHandle, RenderEntityIterator},
    graphics::RenderGroupManager,
    physics::World,
};
use downcast_rs::{impl_downcast, Downcast};
use std::fmt::{Display, Formatter, Result};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityId {
    id: u32,
}

impl EntityId {
    pub const INVALID: Self = Self { id: 0 };
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.id)
    }
}

pub trait EntityIdentifier: Entity {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: EntityId;
    fn entity_type_id(&self) -> EntityId;
}

impl std::hash::Hash for EntityId {
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
        for component in self.components_mut() {
            component.init(handle, world);
        }
    }
    fn finish(&mut self, world: &mut World) {
        for component in self.components_mut() {
            component.finish(world);
        }
    }
    fn tags() -> &'static [&'static str]
    where
        Self: Sized;
    fn components<'a>(&'a self) -> Box<dyn DoubleEndedIterator<Item = &dyn Component> + 'a>;
    fn components_mut<'a>(
        &'a mut self,
    ) -> Box<dyn DoubleEndedIterator<Item = &mut dyn Component> + 'a>;

    fn component<'a>(&'a self, name: &'static str) -> Option<&'a dyn Component>;
    fn component_mut<'a>(&'a mut self, name: &'static str) -> Option<&'a mut dyn Component>;
}
impl_downcast!(Entity);
