#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StateTypeId {
    id: u32,
}

impl StateTypeId {
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

pub trait State {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: StateTypeId;
    const PRIORITY: i16;
}
