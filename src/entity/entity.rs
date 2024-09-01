use downcast_rs::{impl_downcast, Downcast};

use crate::component::Component;
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

pub trait EntityIdentifier: Entity {
    const NAME: &'static str;
    const IDENTIFIER: ConstTypeId =
        ConstTypeId::new(const_fnv1a_hash::fnv1a_hash_str_32(Self::NAME));
    fn const_type_id(&self) -> ConstTypeId {
        Self::IDENTIFIER
    }
}

#[allow(unused_variables)]
pub trait Entity: Downcast + Component {}
impl_downcast!(Entity);
