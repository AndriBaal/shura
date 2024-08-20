use downcast_rs::{impl_downcast, Downcast};

use crate::{component::Component, entity::EntityHandle, physics::World};
use std::fmt::{Display, Formatter, Result};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ConstTypeId {
    id: u32,
}

impl ConstTypeId {
    pub const INVALID: Self = Self { id: 0 };
    pub const fn new(id: u32) -> Self {
        if id == 0 {
            panic!("ConstId cannot be zero!");
        }
        Self { id }
    }
}

impl Display for ConstTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.id)
    }
}

pub trait ConstIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: ConstTypeId =
        ConstTypeId::new(const_fnv1a_hash::fnv1a_hash_str_32(Self::TYPE_NAME));
    fn const_type_id(&self) -> ConstTypeId {
        Self::IDENTIFIER
    }
}

pub trait EntityIdentifier: ConstIdentifier + Entity {}

#[allow(unused_variables)]
pub trait Entity: Downcast {
    fn init(&mut self, handle: EntityHandle, world: &mut World) {}
    fn finish(&mut self, world: &mut World) {}
    fn component(&self, idx: u32) -> Option<&dyn Component> {
        None
    }
    fn component_mut(&mut self, idx: u32) -> Option<&mut dyn Component> {
        None
    }
    // TODO: Optimize with: &'static [u32]
    fn component_identifiers() -> &'static [(ConstTypeId, u32)]
    where
        Self: Sized,
    {
        &[]
    }

    // Optimize with: Vec<Vec<u32>>
    fn component_identifiers_recursive() -> Vec<(ConstTypeId, Vec<u32>)>
    where
        Self: Sized,
    {
        vec![]
    }
}
impl_downcast!(Entity);
