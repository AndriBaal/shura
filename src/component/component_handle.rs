use crate::{ArenaIndex, ComponentGroupId};
use core::hash::Hash;

/// Handle for a component. Through these handles components can be easily be fetches every frame
/// with a specific type through the [component](crate::Context::component) or
/// [component_mut](crate::Context::component_mut) method or without a specific type through the
/// [boxed_component](crate::Context::boxed_component) or
/// [boxed_component_mut](crate::Context::boxed_component_mut) method from the [context](crate::Context)
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentHandle {
    pub(crate) component_index: ArenaIndex,
    type_index: ArenaIndex,
    group_index: ArenaIndex,
    id: u32,
    group_id: ComponentGroupId,
}

impl ComponentHandle {
    pub const INVALID: Self = ComponentHandle {
        component_index: ArenaIndex::INVALID,
        type_index: ArenaIndex::INVALID,
        group_index: ArenaIndex::INVALID,
        id: 0,
        group_id: ComponentGroupId::INVALID,
    };
}

impl Hash for ComponentHandle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        // The id is unique per ComponentHandle, so hashing only the id is faster
        self.id.hash(state)
    }
}

impl Eq for ComponentHandle {}
impl PartialEq for ComponentHandle {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Default for ComponentHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

impl ComponentHandle {
    pub(crate) const fn new(
        component_index: ArenaIndex,
        type_index: ArenaIndex,
        group_index: ArenaIndex,
        id: u32,
        group_id: ComponentGroupId,
    ) -> Self {
        Self {
            id,
            component_index,
            type_index,
            group_index,
            group_id,
        }
    }

    pub(crate) fn type_index(&self) -> ArenaIndex {
        self.type_index
    }

    pub(crate) fn group_index(&self) -> ArenaIndex {
        self.group_index
    }

    pub(crate) fn component_index(&self) -> ArenaIndex {
        self.component_index
    }

    /// Unique if of the handle and its component

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn index(&self) -> u32 {
        self.component_index.index()
    }

    pub fn group_id(&self) -> ComponentGroupId {
        self.group_id
    }
}
