use crate::{
    data::ArenaIndex,
    entity::{EntityIdentifier, EntityTypeId},
};
use core::hash::Hash;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityIndex(pub ArenaIndex);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupHandle(pub ArenaIndex);

impl GroupHandle {
    pub const INVALID: Self = GroupHandle(ArenaIndex::INVALID);
    pub const DEFAULT_GROUP: Self = GroupHandle(ArenaIndex::FIRST);
    pub fn index(&self) -> usize {
        self.0.index()
    }
}

impl EntityIndex {
    pub const INVALID: Self = EntityIndex(ArenaIndex::INVALID);
}

impl Default for GroupHandle {
    fn default() -> Self {
        Self::DEFAULT_GROUP
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityHandle {
    pub entity_index: EntityIndex,
    pub group_handle: GroupHandle,
    pub type_id: EntityTypeId,
}

impl EntityHandle {
    pub const INVALID: Self = EntityHandle {
        entity_index: EntityIndex::INVALID,
        group_handle: GroupHandle::INVALID,
        type_id: EntityTypeId::INVALID,
    };
}

impl Default for EntityHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

impl EntityHandle {
    pub(crate) const fn new(
        entity_index: EntityIndex,
        type_id: EntityTypeId,
        group_handle: GroupHandle,
    ) -> Self {
        Self {
            entity_index,
            type_id,
            group_handle,
        }
    }

    pub fn entity_type_id(&self) -> EntityTypeId {
        self.type_id
    }

    pub fn group_handle(&self) -> GroupHandle {
        self.group_handle
    }

    pub fn type_of<E: EntityIdentifier>(&self) -> bool {
        self.type_id == E::IDENTIFIER
    }

    pub fn index(&self) -> usize {
        self.entity_index.0.index()
    }
}
