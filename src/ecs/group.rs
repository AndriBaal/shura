#[cfg(feature = "serde")]
use crate::{EntityTypeId, FxHashMap};

use crate::{
    data::arena::Arena, Camera2D, EntityManager, GroupHandle, Instant, Vector2, World,
    WorldCamera2D, AABB,
};
use std::fmt;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupManager {
    pub(super) groups: Arena<Group>,
    pub active_size: Vector2<f32>,
}

impl GroupManager {
    pub const DEFAULT_GROUP_NAME: &'static str = "Default Group";
    pub const DEFAULT_GROUP: GroupHandle = GroupHandle::DEFAULT_GROUP;
    pub(crate) fn new() -> Self {
        let default_entity_group =
            Group::new(GroupActivation::Always, 0, Some(Self::DEFAULT_GROUP_NAME));
        let mut groups = Arena::default();
        groups.insert(default_entity_group);
        Self {
            groups,
            active_size: Vector2::new(
                WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV,
                WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV,
            ),
        }
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
            .iter_mut_with_index()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn contains(&self, handle: GroupHandle) -> bool {
        self.groups.contains(handle.0)
    }

    pub fn get(&self, handle: GroupHandle) -> Option<&Group> {
        return self.groups.get(handle.0);
    }

    pub fn get_mut(&mut self, handle: GroupHandle) -> Option<&mut Group> {
        return self.groups.get_mut(handle.0);
    }

    pub fn add(&mut self, entities: &mut EntityManager, group: Group) -> GroupHandle {
        let handle = GroupHandle(self.groups.insert(group));
        for mut ty in entities.types_mut() {
            ty.add_group();
        }
        entities.all_groups.push(handle);
        handle
    }

    pub fn remove(
        &mut self,
        entities: &mut EntityManager,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<Group> {
        if handle == GroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        if group.is_some() {
            entities.active_groups.retain(|g| *g != handle);
            entities.all_groups.retain(|g| *g != handle);
            for mut ty in entities.types_mut() {
                ty.remove_group(world, handle);
            }
            entities.all_groups.retain(|h| *h != handle);
        }
        group
    }

    #[cfg(feature = "serde")]
    pub fn remove_serialize(
        &mut self,
        entities: &mut EntityManager,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<(Group, FxHashMap<EntityTypeId, Box<dyn std::any::Any>>)> {
        if handle == GroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        if let Some(group) = group {
            entities.active_groups.retain(|g| *g != handle);
            entities.all_groups.retain(|g| *g != handle);
            let mut out = FxHashMap::default();
            for mut ty in entities.types_mut() {
                if let Some(g) = ty.remove_group_serialize(world, handle) {
                    out.insert(ty.entity_type_id(), g);
                }
            }
            entities.all_groups.retain(|h| *h != handle);
            Some((group, out))
        } else {
            None
        }
    }

    pub fn update(&mut self, entities: &mut EntityManager, camera: &Camera2D) {
        entities.active_groups.clear();
        let render_aabb = camera.aabb();
        let active_aabb = AABB::from_center(*camera.translation(), self.active_size);
        let now = Instant::now();
        for (index, group) in self.groups.iter_mut_with_index() {
            if group.intersects_aabb(render_aabb) {
                entities.render_groups.push(GroupHandle(index));
            }
            if group.intersects_aabb(active_aabb) {
                group.set_active(true, now);
                entities.active_groups.push(GroupHandle(index));
            }
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum GroupActivation {
    Position { aabb: AABB },
    Always,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Group {
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    active: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "Instant::now"))]
    last_update: Instant,
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
            last_update: Instant::now(),
        }
    }

    pub(crate) fn intersects_aabb(&self, cam_aabb: AABB) -> bool {
        match &self.activation {
            GroupActivation::Position { aabb } => cam_aabb.intersects(aabb),
            GroupActivation::Always => true,
            GroupActivation::Never => false,
        }
    }

    pub(crate) fn set_active(&mut self, active: bool, now: Instant) {
        self.active = active;
        self.last_update = now;
    }

    pub fn set_activation(&mut self, activation: GroupActivation) {
        self.activation = activation;
    }

    pub fn last_update(&self) -> Instant {
        self.last_update
    }

    pub fn set_user_data(&mut self, user_data: u64) {
        self.user_data = user_data;
    }

    pub const fn activation(&self) -> &GroupActivation {
        &self.activation
    }

    pub const fn active(&self) -> bool {
        self.active
    }

    pub const fn user_data(&self) -> u64 {
        self.user_data
    }
}
