use crate::AABB;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone)]
/// Decides when a group is active.
///
/// # Important
/// Components in a inactive [Group] still process the physics
pub enum GroupActivation {
    /// Group is only active when it collides with the fov of the [WorldCamera](crate::WorldCamera)
    Position { aabb: AABB },
    /// Group is always active
    Always,
}

/// Every group has a [id](crate::GroupId) and a [activation](crate::GroupActivation).
/// Groups can be used like a chunk system to make huge 2D worlds possible or to just order your components.
/// The Engine has a default [Group](crate::Group) with the default [GroupId] value.
/// After every update and before rendering, the set of active component groups gets
/// computed. A group can be accessed with [group](crate::Context::group) or
/// [group_mut](crate::Context::group_mut). The components of the group can be accessed with
/// [components](crate::Context::components) or [components_mut](crate::Context::components_mut)
/// from the [context](crate::Context).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Group {
    active: bool,
    pub activation: GroupActivation,
    pub user_data: u64,
}

impl Group {
    pub fn new(activation: GroupActivation, user_data: u64) -> Group {
        Group {
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
