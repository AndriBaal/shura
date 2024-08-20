use crate::{
    arena::Arena,
    entity::{EntityGroupHandle, ConstTypeId, EntityManager, EntityType},
    graphics::Camera2D,
    math::AABB,
    physics::World,
    rustc_hash::FxHashMap,
};
use rstar::{primitives::Rectangle, RTree, RTreeObject};
use rustc_hash::FxHashSet;
use std::{fmt, mem};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityGroupAABB {
    rect: Rectangle<(f32, f32)>,
    pub handle: EntityGroupHandle,
    pub aabb: AABB,
}

impl EntityGroupAABB {
    pub fn new(aabb: AABB, handle: EntityGroupHandle) -> EntityGroupAABB {
        EntityGroupAABB {
            rect: Rectangle::from_corners(
                (aabb.min().x, aabb.min().y),
                (aabb.max().x, aabb.max().y),
            ),
            handle,
            aabb,
        }
    }
}

impl PartialEq for EntityGroupAABB {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl RTreeObject for EntityGroupAABB {
    type Envelope = <Rectangle<(f32, f32)> as RTreeObject>::Envelope;

    fn envelope(&self) -> Self::Envelope {
        self.rect.envelope()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityGroupManager {
    pub(super) groups: Arena<EntityGroup>,
    update_tree: RTree<EntityGroupAABB>,
    render_tree: RTree<EntityGroupAABB>,
    all_groups: FxHashSet<EntityGroupHandle>,
    always_active_groups: FxHashSet<EntityGroupHandle>,
    update_groups: FxHashSet<EntityGroupHandle>,
    render_groups: FxHashSet<EntityGroupHandle>,

    update_groups_changed: bool,
    render_groups_changed: bool,
}

impl EntityGroupManager {
    pub const DEFAULT_GROUP_NAME: &'static str = "Default EntityGroup";
    pub const DEFAULT_GROUP: EntityGroupHandle = EntityGroupHandle::DEFAULT_GROUP;
    pub(crate) fn new() -> Self {
        let default_entity_group = EntityGroup::new(
            GroupActivation::Always,
            GroupActivation::Always,
            0,
            Some(Self::DEFAULT_GROUP_NAME),
        );
        let mut groups = Arena::default();
        groups.insert(default_entity_group);

        let start_group = FxHashSet::from_iter([EntityGroupHandle::DEFAULT_GROUP]);
        Self {
            groups,
            all_groups: start_group.clone(),
            update_groups: start_group.clone(),
            render_groups: start_group.clone(),
            always_active_groups: start_group,
            update_groups_changed: true,
            render_groups_changed: true,
            update_tree: Default::default(),
            render_tree: Default::default(),
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

    pub fn contains(&self, handle: &EntityGroupHandle) -> bool {
        self.groups.contains(handle.0)
    }

    pub fn get(&self, handle: &EntityGroupHandle) -> Option<&EntityGroup> {
        return self.groups.get(handle.0);
    }

    pub fn get_mut(&mut self, handle: &EntityGroupHandle) -> Option<&mut EntityGroup> {
        return self.groups.get_mut(handle.0);
    }

    pub fn add(&mut self, entities: &mut EntityManager, group: EntityGroup) -> EntityGroupHandle {
        let update_activation = group.update_activation;
        let render_activation = group.render_activation;
        let handle = EntityGroupHandle(self.groups.insert(group));
        for mut ty in entities.entities_mut() {
            ty.add_group();
        }
        self.all_groups.insert(handle);

        match update_activation {
            GroupActivation::Position { aabb } => {
                self.update_tree.insert(EntityGroupAABB::new(aabb, handle))
            }
            GroupActivation::Always => {
                self.always_active_groups.insert(handle);
            }
            _ => (),
        }

        if let GroupActivation::Position { aabb } = render_activation {
            self.render_tree.insert(EntityGroupAABB::new(aabb, handle))
        }

        handle
    }

    pub fn remove(
        &mut self,
        entities: &mut EntityManager,
        world: &mut World,
        handle: &EntityGroupHandle,
    ) -> Option<(EntityGroup, FxHashMap<ConstTypeId, Box<dyn EntityType>>)> {
        if *handle == EntityGroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        if let Some(group) = group {
            let mut out = FxHashMap::default();
            for mut ty in entities.entities_mut() {
                if let Some(g) = ty.remove_group(world, handle) {
                    out.insert(ty.entity_type_id(), g);
                }
            }

            // self.update_groups.retain(|h| h != handle);
            // self.render_groups.retain(|h| h != handle);
            self.always_active_groups.remove(handle);
            self.all_groups.remove(handle);
            self.update_tree
                .remove(&EntityGroupAABB::new(Default::default(), *handle)); // Default does not matter, since PartialEq only compares the handle
            self.render_tree
                .remove(&EntityGroupAABB::new(Default::default(), *handle)); // Default does not matter, since PartialEq only compares the handle
            Some((group, out))
        } else {
            None
        }
    }

    pub fn update_tree(&self) -> &RTree<EntityGroupAABB> {
        &self.update_tree
    }

    pub fn render_tree(&self) -> &RTree<EntityGroupAABB> {
        &self.render_tree
    }

    pub(crate) fn update(&mut self, camera: &Camera2D) {
        let aabb = camera.aabb();
        let aabb =
            rstar::AABB::from_corners((aabb.min().x, aabb.min().y), (aabb.max().x, aabb.max().y));

        let old_update_groups =
            mem::replace(&mut self.update_groups, self.always_active_groups.clone());
        let old_render_groups =
            mem::replace(&mut self.render_groups, self.always_active_groups.clone());

        for group in self.render_tree.locate_in_envelope_intersecting(&aabb) {
            self.render_groups.insert(group.handle);
        }

        for group in self.update_tree.locate_in_envelope_intersecting(&aabb) {
            self.update_groups.insert(group.handle);
        }

        self.update_groups_changed = old_update_groups != self.update_groups;
        self.render_groups_changed = old_render_groups != self.render_groups;

        #[cfg(feature = "log")]
        {
            if self.update_groups_changed {
                log::info!("Active groups changed. Now: {}", self.update_groups.len());
            }

            if self.render_groups_changed {
                log::info!("Render groups changed. Now: {}", self.render_groups.len());
            }
        }
    }

    pub fn render_groups(&self) -> &FxHashSet<EntityGroupHandle> {
        &self.render_groups
    }

    pub fn update_groups(&self) -> &FxHashSet<EntityGroupHandle> {
        &self.update_groups
    }

    pub fn render_groups_changed(&self) -> bool {
        self.render_groups_changed
    }

    pub fn update_groups_changed(&self) -> bool {
        self.update_groups_changed
    }

    pub fn all_groups(&self) -> &FxHashSet<EntityGroupHandle> {
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
    // render_active: bool,
    // update_active: bool,
    // #[cfg_attr(feature = "serde", serde(skip))]
    // #[cfg_attr(feature = "serde", serde(default))]
    // last_update: Option<Instant>,
    // #[cfg_attr(feature = "serde", serde(skip))]
    // #[cfg_attr(feature = "serde", serde(default))]
    // last_render: Option<Instant>,
    update_activation: GroupActivation,
    render_activation: GroupActivation,
    pub name: String,
    pub user_data: u64,
}

impl EntityGroup {
    pub fn new(
        update_activation: GroupActivation,
        render_activation: GroupActivation,
        user_data: u64,
        name: Option<&str>,
    ) -> EntityGroup {
        EntityGroup {
            name: name.unwrap_or("EntityGroup").into(),
            update_activation,
            render_activation,
            user_data,
            // last_update: None,
            // last_render: None,
            // render_active: match render_activation {
            //     GroupActivation::Always => true,
            //     _ => false,
            // },
            // update_active: match update_activation {
            //     GroupActivation::Always => true,
            //     _ => false,
            // },
        }
    }

    // pub fn last_update(&self) -> Option<Instant> {
    //     self.last_update
    // }

    // pub fn last_render(&self) -> Option<Instant> {
    //     self.last_render
    // }

    // pub const fn render_active(&self) -> bool {
    //     self.render_active
    // }

    // pub const fn update_active(&self) -> bool {
    //     self.update_active
    // }

    pub fn set_user_data(&mut self, user_data: u64) {
        self.user_data = user_data;
    }

    pub const fn update_activation(&self) -> &GroupActivation {
        &self.update_activation
    }

    pub const fn render_activation(&self) -> &GroupActivation {
        &self.render_activation
    }

    pub const fn user_data(&self) -> u64 {
        self.user_data
    }
}
