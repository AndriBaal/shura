use crate::{ArenaIndex};
use core::hash::Hash;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentIndex(pub(crate) ArenaIndex);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupHandle(pub(crate) ArenaIndex);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TypeIndex(pub(crate)ArenaIndex);

impl GroupHandle {
    pub const DEFAULT_GROUP: Self = GroupHandle(ArenaIndex { index: 0, generation: 0 });
}

impl Default for GroupHandle {
    fn default() -> Self {
        Self::DEFAULT_GROUP
    }
}

/// Handle for a component. Through these handles components can be easily be fetched every frame
/// with a specific type through the [component](crate::Context::component) or
/// [component_mut](crate::Context::component_mut) method or without a specific type through the
/// [boxed_component](crate::Context::boxed_component) or
/// [boxed_component_mut](crate::Context::boxed_component_mut) method from the [context](crate::Context)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentHandle {
    component_index: ComponentIndex,
    type_index: TypeIndex,
    group_index: GroupHandle,
}

impl ComponentHandle {
    pub const INVALID: Self = ComponentHandle {
        component_index: ComponentIndex(ArenaIndex::INVALID),
        type_index: TypeIndex(ArenaIndex::INVALID),
        group_index: GroupHandle(ArenaIndex::INVALID)
    };
}

impl Default for ComponentHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

impl ComponentHandle {
    pub(crate) const fn new(
        component_index: ComponentIndex,
        type_index: TypeIndex,
        group_index: GroupHandle,
    ) -> Self {
        Self {
            component_index,
            type_index,
            group_index,
        }
    }

    pub(crate) fn type_index(&self) -> TypeIndex {
        self.type_index
    }

    pub(crate) fn group_index(&self) -> GroupHandle {
        self.group_index
    }

    pub(crate) fn component_index(&self) -> ComponentIndex {
        self.component_index
    }

    pub fn index(&self) -> u32 {
        self.component_index.0.index()
    }
}
