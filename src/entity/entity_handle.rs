use crate::{
    arena::ArenaIndex,
    component::{Component, ComponentIdentifier},
    entity::{EntityIdentifier, ConstTypeId},
};
use core::hash::Hash;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityIndex(pub ArenaIndex);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityGroupHandle(pub ArenaIndex);

impl EntityGroupHandle {
    pub const INVALID: Self = EntityGroupHandle(ArenaIndex::INVALID);
    pub const DEFAULT_GROUP: Self = EntityGroupHandle(ArenaIndex::FIRST);
    pub fn index(&self) -> usize {
        self.0.index()
    }
}

impl EntityIndex {
    pub const INVALID: Self = EntityIndex(ArenaIndex::INVALID);
}

impl Default for EntityGroupHandle {
    fn default() -> Self {
        Self::DEFAULT_GROUP
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityHandle {
    pub entity_index: EntityIndex,
    pub group_handle: EntityGroupHandle,
    pub type_id: ConstTypeId,
}

impl EntityHandle {
    pub const INVALID: Self = EntityHandle {
        entity_index: EntityIndex::INVALID,
        group_handle: EntityGroupHandle::INVALID,
        type_id: ConstTypeId::INVALID,
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
        type_id: ConstTypeId,
        group_handle: EntityGroupHandle,
    ) -> Self {
        Self {
            entity_index,
            type_id,
            group_handle,
        }
    }

    pub fn is<E: EntityIdentifier>(&self) -> bool {
        E::IDENTIFIER == self.entity_type_id()
    }

    pub fn entity_type_id(&self) -> ConstTypeId {
        self.type_id
    }

    pub fn group_handle(&self) -> EntityGroupHandle {
        self.group_handle
    }

    pub fn type_of<E: EntityIdentifier>(&self) -> bool {
        self.type_id == E::IDENTIFIER
    }

    pub fn index(&self) -> usize {
        self.entity_index.0.index()
    }
}

impl ComponentIdentifier for EntityHandle {
    const NAME: &'static str = concat!(module_path!(), "ColliderComponent");
}

impl Component for EntityHandle {
    fn init(&mut self, handle: EntityHandle, _world: &mut crate::physics::World) {
        *self = handle;
    }

    fn finish(&mut self, _world: &mut crate::physics::World) {
        *self = Self::INVALID;
    }

    fn as_component(&self) -> &dyn Component {
        self as _
    }
    
    fn as_component_mut(&mut self) -> &mut dyn Component {
        self as _
    }
}
