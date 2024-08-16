use downcast_rs::{impl_downcast, Downcast};

use crate::component::Component;
use std::fmt::{Display, Formatter, Result};

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

pub trait Entity: Component + Downcast {}
impl_downcast!(Entity);
