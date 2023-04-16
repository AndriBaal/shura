use crate::data::arena::{Arena, ArenaIndex, ArenaIterMut};
use crate::{ComponentController, ComponentType, ComponentTypeId, Vector};
use rustc_hash::FxHashMap;

/// Helper to create a [ComponentGroup](crate::ComponentGroup).
#[derive(Debug, Copy, Clone)]
pub struct ComponentGroupDescriptor {
    /// Id of the group.
    pub id: u16,
    /// Describes when the ggroup is active.
    pub activation: GroupActivation,
    /// Describes if the group is enabled from the start.
    pub user_data: u64,
}

/// Id of the default [ComponentGroup](crate::ComponentGroup). Components within this group are
/// always getting rendered and updated in every cycle.
pub const DEFAULT_GROUP_ID: u16 = u16::MAX / 2;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum GroupActivation {
    Position {
        position: Vector<f32>,
        half_extents: Vector<f32>,
    },
    Always,
}

impl Into<ComponentGroup> for ComponentGroupDescriptor {
    fn into(self) -> ComponentGroup {
        ComponentGroup {
            id: self.id,
            activation: self.activation,
            type_map: Default::default(),
            types: Default::default(),
            active: false,
            user_data: self.user_data,
        }
    }
}

/// Every group has a id and a fixed position where it operates. When the camera intersects with
/// the position and size of the group the group is marked as `active`.It can be used like a chunk
/// system to make huge 2D worlds possible or to just order your components. The Engine has a
/// default [ComponentGroup](crate::ComponentGroup) that holds the [DEFAULT_GROUP_ID]. After every update and before rendering, the set of active component groups gets
/// computed. A group can be accessed with [group](crate::Context::group) or
/// [group_mut](crate::Context::group_mut). The components of the group can be accessed with
/// [components](crate::Context::components) or [components_mut](crate::Context::components_mut)
/// from the [context](crate::Context).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ComponentGroup {
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    type_map: FxHashMap<ComponentTypeId, ArenaIndex>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    types: Arena<ComponentType>,
    id: u16,
    active: bool,
    pub activation: GroupActivation,
    pub user_data: u64,
}

impl ComponentGroup {
    pub(crate) fn intersects_camera(
        &self,
        cam_bottom_left: Vector<f32>,
        cam_top_right: Vector<f32>,
    ) -> bool {
        match &self.activation {
            GroupActivation::Position {
                position,
                half_extents,
            } => {
                let self_bl = Vector::new(position.x - half_extents.x, position.y - half_extents.y);
                let self_tr = Vector::new(position.x + half_extents.x, position.y + half_extents.y);
                return (cam_bottom_left.x < self_tr.x)
                    && (self_bl.x < cam_top_right.x)
                    && (cam_bottom_left.y < self_tr.y)
                    && (self_bl.y < cam_top_right.y);
            }
            GroupActivation::Always => {
                return true;
            }
        }
    }

    pub(crate) fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Set the activation of this group.
    pub fn set_activation(&mut self, activation: GroupActivation) {
        self.activation = activation;
    }

    pub fn set_user_data(&mut self, user_data: u64) {
        self.user_data = user_data;
    }

    pub(crate) fn type_index(&self, type_id: ComponentTypeId) -> Option<&ArenaIndex> {
        self.type_map.get(&type_id)
    }

    pub(crate) fn type_ref(&self, index: ArenaIndex) -> Option<&ComponentType> {
        self.types.get(index)
    }

    pub(crate) fn type_mut(&mut self, index: ArenaIndex) -> Option<&mut ComponentType> {
        self.types.get_mut(index)
    }

    pub(crate) fn types(&mut self) -> ArenaIterMut<ComponentType> {
        self.types.iter_mut()
    }

    pub(crate) fn remove_type(&mut self, index: ArenaIndex) {
        let removed = self.types.remove(index).unwrap();
        self.type_map.remove(&removed.type_id());
    }

    pub(crate) fn add_component_type<C: ComponentController>(
        &mut self,
        component: C,
    ) -> (ArenaIndex, ArenaIndex) {
        let (component_index, component_type) = ComponentType::new(component);
        let type_index = self.types.insert(component_type);
        self.type_map.insert(C::IDENTIFIER, type_index);
        return (type_index, component_index);
    }

    /// Get the id of the group.
    pub const fn id(&self) -> u16 {
        self.id
    }

    /// Get the activation of this Group.
    pub const fn activation(&self) -> &GroupActivation {
        &self.activation
    }

    /// See if this group is active in the current cycle.
    pub const fn active(&self) -> bool {
        self.active
    }

    pub const fn user_data(&self) -> u64 {
        self.user_data
    }
}
