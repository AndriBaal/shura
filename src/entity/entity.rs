use crate::{
    component::Component,
    entity::{EntityHandle, RenderEntityIterator},
    graphics::ComponentBufferManager,
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
        buffers: &mut ComponentBufferManager,
        world: &World,
    ) where
        Self: Sized;
    // fn components(&self) -> impl Iterator<Item = &dyn Component>
    // where
    //     Self: Sized;
    // fn components_mut(&mut self) -> impl Iterator<Item = &mut dyn Component>
    // where
    //     Self: Sized;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
    fn components_dyn<'a>(&'a self) -> Box<dyn Iterator<Item = &dyn Component> + 'a>;
}
impl_downcast!(Entity);
