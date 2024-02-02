#[cfg(feature = "serde")]
use crate::{
    entity::{EntityType, EntityTypeId},
    rustc_hash::FxHashMap,
};

use crate::{
    data::Arena,
    entity::{EntityGroupHandle, EntityManager},
    graphics::{Camera2D, WorldCamera2D},
    math::{Vector2, AABB},
    physics::World,
    time::Instant,
};
use std::fmt;
use std::mem::swap;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityGroupManager {
    pub(super) groups: Arena<EntityGroup>,
    pub active_size: Vector2<f32>,
    all_groups: Vec<EntityGroupHandle>,
    active_groups_changed: bool,
    render_groups_changed: bool,

    active_groups: Vec<EntityGroupHandle>,
    render_groups: Vec<EntityGroupHandle>,

    last_active_groups: Vec<EntityGroupHandle>,
    last_render_groups: Vec<EntityGroupHandle>,
}

impl EntityGroupManager {
    pub const DEFAULT_GROUP_NAME: &'static str = "Default EntityGroup";
    pub const DEFAULT_GROUP: EntityGroupHandle = EntityGroupHandle::DEFAULT_GROUP;
    pub(crate) fn new() -> Self {
        let default_entity_group =
            EntityGroup::new(GroupActivation::Always, 0, Some(Self::DEFAULT_GROUP_NAME));
        let mut groups = Arena::default();
        groups.insert(default_entity_group);
        Self {
            groups,
            all_groups: Vec::from_iter([EntityGroupHandle::DEFAULT_GROUP]),
            active_groups: Vec::from_iter([EntityGroupHandle::DEFAULT_GROUP]),
            render_groups: Vec::from_iter([EntityGroupHandle::DEFAULT_GROUP]),
            active_size: Vector2::new(
                WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV,
                WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV,
            ),
            last_active_groups: Default::default(),
            last_render_groups: Default::default(),
            active_groups_changed: true,
            render_groups_changed: true,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (EntityGroupHandle, &EntityGroup)> + Clone {
        return self
            .groups
            .iter_with_index()
            .map(|(index, group)| (EntityGroupHandle(index), group));
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityGroupHandle, &mut EntityGroup)> {
        return self
            .groups
            .iter_mut_with_index()
            .map(|(index, group)| (EntityGroupHandle(index), group));
    }

    pub fn contains(&self, handle: EntityGroupHandle) -> bool {
        self.groups.contains(handle.0)
    }

    pub fn get(&self, handle: EntityGroupHandle) -> Option<&EntityGroup> {
        return self.groups.get(handle.0);
    }

    pub fn get_mut(&mut self, handle: EntityGroupHandle) -> Option<&mut EntityGroup> {
        return self.groups.get_mut(handle.0);
    }

    pub fn add(&mut self, entities: &mut EntityManager, group: EntityGroup) -> EntityGroupHandle {
        let handle = EntityGroupHandle(self.groups.insert(group));
        for mut ty in entities.types_mut() {
            ty.add_group();
        }
        self.all_groups.push(handle);
        handle
    }

    pub fn remove(
        &mut self,
        entities: &mut EntityManager,
        world: &mut World,
        handle: EntityGroupHandle,
    ) -> Option<EntityGroup> {
        if handle == EntityGroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        if group.is_some() {
            self.active_groups.retain(|g| *g != handle);
            self.all_groups.retain(|g| *g != handle);
            for mut ty in entities.types_mut() {
                ty.remove_group(world, handle);
            }
            self.all_groups.retain(|h| *h != handle);
        }
        group
    }

    #[cfg(feature = "serde")]
    pub fn remove_serialize(
        &mut self,
        entities: &mut EntityManager,
        world: &mut World,
        handle: EntityGroupHandle,
    ) -> Option<(EntityGroup, FxHashMap<EntityTypeId, Box<dyn EntityType>>)> {
        if handle == EntityGroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        if let Some(group) = group {
            self.active_groups.retain(|g| *g != handle);
            self.all_groups.retain(|g| *g != handle);
            let mut out = FxHashMap::default();
            for mut ty in entities.types_mut() {
                if let Some(g) = ty.remove_group(world, handle) {
                    out.insert(ty.entity_type_id(), g);
                }
            }
            self.all_groups.retain(|h| *h != handle);
            Some((group, out))
        } else {
            None
        }
    }

    pub(crate) fn update(&mut self, camera: &Camera2D) {
        self.last_render_groups.clear();
        self.last_active_groups.clear();
        swap(&mut self.active_groups, &mut self.last_active_groups);
        swap(&mut self.render_groups, &mut self.last_render_groups);

        let render_aabb = camera.aabb();
        let active_aabb = AABB::from_center(*camera.translation(), self.active_size);
        let now = Instant::now();
        self.active_groups_changed = false;
        self.render_groups_changed = false;
        for (index, group) in self.groups.iter_mut_with_index() {
            if group.intersects_aabb(render_aabb) {
                self.render_groups.push(EntityGroupHandle(index));
                let i = self.render_groups.len() - 1;
                if self.render_groups[i]
                    != self.last_render_groups.get(i).cloned().unwrap_or_default()
                {
                    self.render_groups_changed = true;
                }
            }

            if group.intersects_aabb(active_aabb) {
                group.set_active(true, now);
                self.active_groups.push(EntityGroupHandle(index));
                let i = self.active_groups.len() - 1;
                if self.active_groups[i]
                    != self.last_active_groups.get(i).cloned().unwrap_or_default()
                {
                    self.active_groups_changed = true;
                }
            }
        }

        #[cfg(feature = "log")]
        {
            if self.active_groups_changed {
                log::info!("Active groups changed. Now: {}", self.active_groups.len());
            }

            if self.render_groups_changed {
                log::info!("Render groups changed. Now: {}", self.render_groups.len());
            }
        }
    }

    pub fn render_groups(&self) -> &[EntityGroupHandle] {
        &self.render_groups
    }

    pub fn active_groups(&self) -> &[EntityGroupHandle] {
        &self.active_groups
    }

    pub fn render_groups_changed(&self) -> bool {
        self.render_groups_changed
    }

    pub fn active_groups_changed(&self) -> bool {
        self.active_groups_changed
    }

    pub fn all_groups(&self) -> &[EntityGroupHandle] {
        &self.all_groups
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
pub struct EntityGroup {
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

impl EntityGroup {
    pub fn new(activation: GroupActivation, user_data: u64, name: Option<&str>) -> EntityGroup {
        EntityGroup {
            name: name.unwrap_or("EntityGroup").into(),
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
