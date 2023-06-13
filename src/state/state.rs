use downcast_rs::{impl_downcast, Downcast};

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// TypeId of a struct that derives from the [state](crate::State) macro. The diffrence to the [std::any::TypeId] is, that
/// this TypeId is const and is the same on every system.
///
/// # How it works
/// It works by providing a unique identifier to the derive macro. This unique identifier can be passed
/// with the `name` attribute, otherwise it is just the struct name. Then this identifier is hashed to a unique
/// u32. The macro is checking at compile time, that every [StateTypeId] is unique.
pub struct StateTypeId {
    id: u32,
}

impl StateTypeId {
    pub const fn new(id: u32) -> Self {
        Self { id }
    }
}

/// Trait to identify a struct that derives from  the [State](crate::State) macro using
/// a [StateTypeId]
pub trait StateIdentifier {
    const TYPE_NAME: &'static str;
    const IDENTIFIER: StateTypeId;
}

pub trait StateDerive: Downcast {}
impl_downcast!(StateDerive);
