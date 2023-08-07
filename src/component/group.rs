use crate::{data::arena::Arena, CameraBuffer, ComponentManager, GroupHandle, Vector, AABB};
use std::fmt;

#[cfg(feature = "physics")]
use crate::physics::World;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupManager {
    pub(super) groups: Arena<Group>,
}

impl GroupManager {
    pub const DEFAULT_GROUP_NAME: &str = "Default Group";
    pub const DEFAULT_GROUP: GroupHandle = GroupHandle::DEFAULT_GROUP;
    pub(crate) fn new() -> Self {
        let default_component_group =
            Group::new(GroupActivation::Always, 0, Some(Self::DEFAULT_GROUP_NAME));
        let mut groups = Arena::default();
        groups.insert(default_component_group);
        Self { groups }
    }

    pub fn iter(&self) -> impl Iterator<Item = (GroupHandle, &Group)> + Clone {
        return self
            .groups
            .iter_with_index()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (GroupHandle, &mut Group)> {
        return self
            .groups
            .iter_with_index_mut()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn contains(&self, handle: GroupHandle) -> bool {
        return self.groups.contains(handle.0);
    }

    pub fn get(&self, handle: GroupHandle) -> Option<&Group> {
        return self.groups.get(handle.0);
    }

    pub fn get_mut(&mut self, handle: GroupHandle) -> Option<&mut Group> {
        return self.groups.get_mut(handle.0);
    }

    pub fn add(&mut self, components: &mut ComponentManager, group: Group) -> GroupHandle {
        let handle = GroupHandle(self.groups.insert(group));
        for mut ty in components.types_mut() {
            ty.add_group();
        }
        components.all_groups.push(handle);
        return handle;
    }

    pub fn remove(
        &mut self,
        components: &mut ComponentManager,
        #[cfg(feature = "physics")] world: &mut World,
        handle: GroupHandle,
    ) -> Option<Group> {
        if handle == GroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        components.active_groups.retain(|g| *g != handle);
        components.all_groups.retain(|g| *g != handle);
        for mut ty in components.types_mut() {
            ty.remove_group(
                #[cfg(feature = "physics")]
                world,
                handle,
            );
        }
        components.all_groups.retain(|h| *h != handle);
        return group;
    }

    pub(crate) fn update(&mut self, components: &mut ComponentManager, camera: &CameraBuffer) {
        let cam_aabb = camera.model().aabb(Vector::new(0.0, 0.0).into()); // Translation is already applied
        components.active_groups.clear();
        for (index, group) in self.groups.iter_with_index_mut() {
            if group.intersects_camera(cam_aabb) {
                group.set_active(true);
                components.active_groups.push(GroupHandle(index));
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
/// Decides when a group is active.
///
/// # Important
/// Components in a inactive [Group] still physics events
pub enum GroupActivation {
    /// Group is only active when it collides with the fov of the [WorldCamera](crate::WorldCamera)
    Position { aabb: AABB },
    /// Group is always active
    Always,
    /// Never
    Never,
}

impl fmt::Display for GroupActivation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupActivation::Position { .. } => f.write_str("Position"),
            GroupActivation::Always => f.write_str("Always"),
            GroupActivation::Never => f.write_str("Never"),
        }
    }
}
/// Groups can be used like a chunk system to make huge 2D worlds possible or to just order your components.
/// The Engine has a default [Group](crate::Group) with the [default handle](crate::GroupHandle::DEFAULT_GROUP).
/// After every update and before rendering, the set of active groups gets
/// computed.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Group {
    active: bool,
    pub name: String,
    pub activation: GroupActivation,
    pub user_data: u64,
}

impl Group {
    pub fn new(activation: GroupActivation, user_data: u64, name: Option<&str>) -> Group {
        Group {
            name: name.unwrap_or("Group").into(),
            activation,
            user_data,
            active: false,
        }
    }

    pub(crate) fn intersects_camera(&self, cam_aabb: AABB) -> bool {
        match &self.activation {
            GroupActivation::Position { aabb } => return cam_aabb.intersects(aabb),
            GroupActivation::Always => {
                return true;
            }
            GroupActivation::Never => {
                return false;
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
