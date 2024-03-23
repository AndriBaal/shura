use crate::component::ComponentBundle;
use std::fmt::{Display, Formatter, Result};

pub trait BufferEntityIterator<'a, E: Entity>: Iterator<Item = &'a E> + Clone + 'a {}
impl<'a, E: Entity, I: Iterator<Item = &'a E> + Clone + 'a> BufferEntityIterator<'a, E> for I {}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default, Hash)]
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

pub trait Entity: ComponentBundle {}
