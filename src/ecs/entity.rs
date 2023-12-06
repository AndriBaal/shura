use downcast_rs::{Downcast, impl_downcast};

use crate::{ComponentBufferManager, EntityHandle, Instance, World, RenderEntityIterator};
use std::fmt::{Display, Formatter, Result};

#[allow(unused_variables)]
pub trait Component {
    type Instance: Instance;
    fn instance(&self, world: &World) -> Self::Instance;
    fn active(&self) -> bool;
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
}

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
    ) where Self: Sized;
    fn components(&self) -> Vec<&dyn std::any::Any>;
    fn init(&mut self, handle: EntityHandle, world: &mut World);
    fn finish(&mut self, world: &mut World);
}
impl_downcast!(Entity);
