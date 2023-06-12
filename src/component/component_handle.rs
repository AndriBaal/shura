use crate::ArenaIndex;
use core::hash::Hash;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct ComponentIndex(pub(crate) ArenaIndex);
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Unique identifier of a group
pub struct GroupHandle(pub(crate) ArenaIndex);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct TypeIndex(pub(crate) ArenaIndex);

impl GroupHandle {
    pub const DEFAULT_GROUP: Self = GroupHandle(ArenaIndex::FIRST);
}

impl Default for GroupHandle {
    fn default() -> Self {
        Self::DEFAULT_GROUP
    }
}

/// Handle for a component. Through these handles components can be easily be fetched every frame
/// with a specific type through the [component](crate::ComponentManager::get) or
/// [component_mut](crate::ComponentManager::get_mut) method or without a specific type through the
/// [boxed_component](crate::ComponentManager::get_boxed) or
/// [boxed_component_mut](crate::ComponentManager::get_boxed_mut) method from the [context](crate::Context)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentHandle {
    component_index: ComponentIndex,
    type_index: TypeIndex,
    group_handle: GroupHandle,
}

impl ComponentHandle {
    pub const INVALID: Self = ComponentHandle {
        component_index: ComponentIndex(ArenaIndex::INVALID),
        type_index: TypeIndex(ArenaIndex::INVALID),
        group_handle: GroupHandle(ArenaIndex::INVALID),
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
        group_handle: GroupHandle,
    ) -> Self {
        Self {
            component_index,
            type_index,
            group_handle,
        }
    }

    pub(crate) fn type_index(&self) -> TypeIndex {
        self.type_index
    }

    pub fn group_handle(&self) -> GroupHandle {
        self.group_handle
    }

    pub(crate) fn component_index(&self) -> ComponentIndex {
        self.component_index
    }

    pub fn index(&self) -> usize {
        self.component_index.0.index()
    }
}
