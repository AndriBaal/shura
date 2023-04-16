#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Arena, ArenaEntry, ArenaIndex, ArenaPath, BoxedComponent, CameraBuffer, ComponentCallbacks,
    ComponentCluster, ComponentController, ComponentDerive, ComponentGroup,
    ComponentGroupDescriptor, ComponentHandle, ComponentIterRender, ComponentPath,
    ComponentRenderGroup, ComponentSet, ComponentSetMut, ComponentTypeId, Gpu, GpuDefaults,
    GroupActivation, InstanceBuffer, Vector, DEFAULT_GROUP_ID,
};
use instant::Instant;
#[cfg(feature = "log")]
use log::info;
use rustc_hash::{FxHashMap, FxHashSet};
#[cfg(feature = "physics")]
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::rc::Rc;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GroupDelta {
    Add(u16),
    Remove(u16)
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum GroupFilter<'a> {
    All,
    Active,
    Specific(&'a [u16]),
}

impl<'a> Default for GroupFilter<'a> {
    fn default() -> Self {
        return GroupFilter::DEFAULT_GROUP;
    }
}

impl GroupFilter<'static> {
    pub const DEFAULT_GROUP: Self = GroupFilter::Specific(&[DEFAULT_GROUP_ID]);
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Access to the component system.
pub struct ComponentManager {
    render_components: bool,
    id_counter: u32,
    force_update_sets: bool,
    group_map: FxHashMap<u16, ArenaIndex>,
    group_deltas: Vec<GroupDelta>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    groups: Arena<ComponentGroup>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    active_groups: FxHashSet<ArenaIndex>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    active_group_ids: Vec<u16>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    active_components: Rc<BTreeMap<(i16, ComponentTypeId), ComponentCluster>>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    component_callbacks: FxHashMap<ComponentTypeId, ComponentCallbacks>,
    #[cfg(feature = "physics")]
    pub(crate) world: Rc<RefCell<World>>,
}

impl ComponentManager {
    pub(crate) fn new() -> Self {
        let default_component_group = ComponentGroupDescriptor {
            id: DEFAULT_GROUP_ID,
            activation: GroupActivation::Always,
            user_data: 0,
        }.into();
        let mut groups = Arena::default();
        let mut group_map = FxHashMap::default();
        let index = groups.insert(default_component_group);
        group_map.insert(DEFAULT_GROUP_ID, index);
        Self {
            active_groups: FxHashSet::from_iter([index]),
            active_group_ids: vec![DEFAULT_GROUP_ID],
            groups,
            group_map,
            group_deltas: vec![],

            render_components: true,

            id_counter: 0,
            force_update_sets: false,
            active_components: Default::default(),
            component_callbacks: Default::default(),

            #[cfg(feature = "physics")]
            world: Rc::new(RefCell::new(World::new())),
        }
    }

    pub(crate) fn update_sets(&mut self, camera: &CameraBuffer) {
        let aabb = camera.model().aabb(Vector::new(0.0, 0.0).into());
        let active_components = Rc::get_mut(&mut self.active_components).unwrap();
        let now = Instant::now();
        let mut groups_changed = false;
        self.group_deltas.clear();
        for (index, group) in &mut self.groups {
            if group.intersects_camera(aabb.0, aabb.1) {
                group.set_active(true);
                if self.active_groups.insert(index) {
                    self.group_deltas.push(GroupDelta::Add(group.id()));
                    groups_changed = true;
                }
            } else {
                group.set_active(false);
                if self.active_groups.remove(&index) {
                    self.group_deltas.push(GroupDelta::Remove(group.id()));
                    groups_changed = true;
                }
            }
        }

        if self.force_update_sets || groups_changed {
            #[cfg(feature = "log")]
            {
                info!("Rebuilding Active Components");
                info!("Now processing {} group(s)", self.active_groups.len());
            }
            self.force_update_sets = false;
            for set in active_components.values_mut() {
                set.clear();
            }
            for index in &self.active_groups {
                let group = self.groups.get_mut(*index).unwrap();
                for (type_index, component_type) in group.types() {
                    if component_type.is_empty() {
                        continue;
                    }
                    let type_id = component_type.type_id();
                    let priority = component_type.config().priority;
                    let key = (priority, type_id);
                    let path = ArenaPath {
                        group_index: *index,
                        type_index,
                    };
                    if let Some(active_component) = active_components.get_mut(&key) {
                        active_component.add(path);
                        active_component.update_time(now);
                    } else {
                        let config = component_type.config();
                        active_components.insert(
                            key,
                            ComponentCluster::new(
                                path,
                                self.component_callbacks.get(&type_id).unwrap().clone(),
                                config.clone(),
                                now,
                            ),
                        );
                    }
                }
            }
            for cluster in active_components.values_mut() {
                cluster.sort(); // Sorting needed for components_mut
            }
            self.active_group_ids = self
                .active_groups
                .iter()
                .map(|i| self.groups[*i].id())
                .collect();
        }
    }

    pub(crate) fn buffer_sets(&mut self, gpu: &Gpu) {
        for group in &self.active_groups {
            if let Some(group) = self.groups.get_mut(*group) {
                for (_, t) in group.types() {
                    t.buffer_data(gpu);
                }
            }
        }
    }

    pub fn force_buffer<C: ComponentController>(&mut self, filter: GroupFilter) {
        let type_id = C::IDENTIFIER;
        match filter {
            GroupFilter::All => {
                for group in &mut self.groups {
                    if let Some(index) = group.1.type_index(type_id) {
                        let component_type = group.1.type_mut(*index).unwrap();
                        component_type.set_force_buffer(true);
                    }
                }
            }
            GroupFilter::Active => {
                for group in self.active_groups.iter() {
                    if let Some(group) = self.groups.get_mut(*group) {
                        if let Some(index) = group.type_index(type_id) {
                            let component_type = group.type_mut(*index).unwrap();
                            component_type.set_force_buffer(true);
                        }
                    }
                }
            }
            GroupFilter::Specific(groups) => {
                for group_id in groups {
                    if let Some(group_index) = self.group_map.get(group_id) {
                        let group = &mut self.groups[*group_index];
                        if let Some(index) = group.type_index(type_id) {
                            let component_type = group.type_mut(*index).unwrap();
                            component_type.set_force_buffer(true);
                        }
                    }
                }
            }
        }
    }

    pub fn add_component<C: ComponentController>(
        &mut self,
        component: C,
    ) -> (&mut C, ComponentHandle) {
        return self.add_component_with_group(None, component);
    }

    pub fn add_component_with_group<C: ComponentController>(
        &mut self,
        group_id: Option<u16>,
        component: C,
    ) -> (&mut C, ComponentHandle) {
        let group_id = group_id.unwrap_or(DEFAULT_GROUP_ID);
        let type_id = C::IDENTIFIER;
        let group_index = self
            .group_map
            .get(&group_id)
            .expect(format!("Group {} does not exist!", group_id).as_str());
        let group = &mut self.groups[*group_index];

        self.component_callbacks
            .insert(type_id, ComponentCallbacks::new::<C>());

        let handle;
        if let Some(type_index) = group.type_index(type_id).cloned() {
            self.id_counter += 1;
            let component_type = group.type_mut(type_index).unwrap();
            let index = component_type.add(component);
            handle =
                ComponentHandle::new(index, type_index, *group_index, self.id_counter, group_id);
        } else {
            // Create a new ComponentType
            self.id_counter += 1;
            self.force_update_sets = true;
            let (type_index, index) = group.add_component_type(component);
            handle =
                ComponentHandle::new(index, type_index, *group_index, self.id_counter, group_id);
        }

        let c = self
            .groups
            .get_mut(handle.group_index())
            .unwrap()
            .type_mut(handle.type_index())
            .unwrap()
            .component_mut(handle.component_index())
            .unwrap();
        c.base_mut().init(handle);
        #[cfg(feature = "physics")]
        if c.base().is_rigid_body() {
            c.base_mut().add_to_world(C::IDENTIFIER, self.world.clone())
        }
        return (c.downcast_mut().unwrap(), handle);
    }

    pub fn add_group(&mut self, group: impl Into<ComponentGroup>) {
        let mut group = group.into();
        let group_id = group.id();
        assert!(self.group_map.contains_key(&group_id) == false);
        #[cfg(feature = "physics")]
        for (_, component_type) in group.types() {
            let type_id = component_type.type_id();
            for (_, component) in component_type {
                component.base_mut().add_to_world(type_id, self.world.clone())
            }
        }
        let index = self.groups.insert(group);
        self.force_update_sets = true;
        self.group_map.insert(group_id, index);
    }

    pub fn remove_component(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
        if let Some(group) = self.groups.get_mut(handle.group_index()) {
            let type_index = handle.type_index();
            if let Some(component_type) = group.type_mut(type_index) {
                if let Some(mut to_remove) = component_type.remove(handle) {
                    to_remove.base_mut().deinit();
                    if component_type.len() == 0 {
                        self.force_update_sets = false;
                        group.remove_type(type_index);
                    }
                    return Some(to_remove);
                }
            }
        }
        return None;
    }

    pub fn remove_components<C: ComponentController>(&mut self, filter: GroupFilter) {
        let type_id = C::IDENTIFIER;

        fn remove(rewrite: &mut bool, group: &mut ComponentGroup, type_id: ComponentTypeId) {
            if let Some(type_index) = group.type_index(type_id).cloned() {
                let component_type = group.type_mut(type_index).unwrap();
                for (_, c) in component_type.iter_mut() {
                    c.base_mut().deinit();
                }
                if component_type.len() == 0 {
                    group.remove_type(type_index);
                    *rewrite = true;
                }
            }
        }

        match filter {
            GroupFilter::All => {
                for (_index, group) in &mut self.groups {
                    remove(&mut self.force_update_sets, group, type_id)
                }
            }
            GroupFilter::Active => {
                for index in &self.active_groups {
                    if let Some(group) = self.groups.get_mut(*index) {
                        remove(&mut self.force_update_sets, group, type_id)
                    }
                }
            }
            GroupFilter::Specific(group_ids) => {
                for group_id in group_ids {
                    if let Some(index) = self.group_map.get(&group_id) {
                        let group = self.groups.get_mut(*index).unwrap();
                        remove(&mut self.force_update_sets, group, type_id)
                    }
                }
            }
        }
    }

    pub fn remove_group(&mut self, group_id: u16) -> Option<ComponentGroup> {
        if group_id == DEFAULT_GROUP_ID {
            return None;
        }

        if let Some(index) = self.group_map.remove(&group_id) {
            #[cfg(feature = "physics")]
            if let Some(mut group) = self.groups.remove(index) {
                for (_, component_type) in group.types() {
                    for (_, c) in component_type.iter_mut() {
                        c.base_mut().deinit();
                    }
                }
            }

            self.force_update_sets = true;
            self.active_groups.remove(&index);
            return self.groups.remove(index);
        }
        return None;
    }

    pub fn path_render<'a, C: ComponentDerive>(
        &'a self,
        path: &ComponentPath<C>,
        defaults: &'a GpuDefaults,
    ) -> ComponentRenderGroup<'a, C> {
        let mut iters = vec![];
        let mut len = 0;

        for path in path.paths() {
            if let Some(group) = self.group(path.group_index) {
                if let Some(component_type) = group.type_ref(path.type_index) {
                    let type_len = component_type.len();
                    if type_len > 0 {
                        len += type_len;
                        iters.push((
                            component_type.buffer().unwrap_or(&defaults.empty_instance),
                            ComponentIterRender::new(component_type.iter().enumerate()),
                        ));
                    }
                }
            }
        }

        return ComponentRenderGroup::new(iters, len);
    }

    pub fn path<'a, C: ComponentDerive>(&'a self, path: &ComponentPath<C>) -> ComponentSet<'a, C> {
        let mut iters = vec![];
        let mut len = 0;

        for path in path.paths() {
            if let Some(group) = self.group(path.group_index) {
                if let Some(component_type) = group.type_ref(path.type_index) {
                    let type_len = component_type.len();
                    if type_len > 0 {
                        len += type_len;
                        iters.push(component_type.iter());
                    }
                }
            }
        }

        return ComponentSet::new(iters, len);
    }

    pub fn path_mut<'a, C: ComponentDerive>(
        &'a mut self,
        path: &ComponentPath<C>,
    ) -> ComponentSetMut<'a, C> {
        let mut iters = vec![];
        let mut len = 0;

        let mut head: &mut [ArenaEntry<_>] = self.groups.as_slice();
        let mut offset = 0;
        for path in path.paths() {
            let split = head.split_at_mut(path.group_index.index() as usize + 1 - offset);
            head = split.1;
            offset += split.0.len();
            match split.0.last_mut().unwrap() {
                ArenaEntry::Occupied { data, .. } => {
                    if let Some(component_type) = data.type_mut(path.type_index) {
                        let type_len = component_type.len();
                        if type_len > 0 {
                            len += type_len;
                            iters.push(component_type.iter_mut());
                        }
                    }
                }
                _ => unreachable!(),
            };
        }

        return ComponentSetMut::new(iters, len);
    }

    pub fn components<'a, C: ComponentController>(
        &'a self,
        filter: GroupFilter,
    ) -> ComponentSet<'a, C> {
        let type_id = C::IDENTIFIER;
        let mut iters = vec![];
        let mut len = 0;

        match filter {
            GroupFilter::All => {
                for (_, group) in &self.groups {
                    if let Some(type_index) = group.type_index(type_id) {
                        let component_type = group.type_ref(*type_index).unwrap();
                        let type_len = component_type.len();
                        if type_len > 0 {
                            len += type_len;
                            iters.push(component_type.iter());
                        }
                    }
                }
            }
            GroupFilter::Active => {
                let key = (C::CONFIG.priority, C::IDENTIFIER);
                if let Some(cluster) = self.active_components.get(&key) {
                    return self.path(&ComponentPath::new(cluster.paths()));
                }
            }
            GroupFilter::Specific(group_ids) => {
                for group_id in group_ids {
                    if let Some(index) = self.group_map.get(&group_id) {
                        let group = self.groups.get(*index).unwrap();

                        if let Some(type_index) = group.type_index(type_id) {
                            let component_type = group.type_ref(*type_index).unwrap();
                            let type_len = component_type.len();
                            if type_len > 0 {
                                len += type_len;
                                iters.push(component_type.iter());
                            }
                        };
                    }
                }
            }
        };

        return ComponentSet::new(iters, len);
    }

    pub fn components_mut<C: ComponentController>(
        &mut self,
        filter: GroupFilter,
    ) -> ComponentSetMut<C> {
        let type_id = C::IDENTIFIER;
        let mut iters = vec![];
        let mut len = 0;

        match filter {
            GroupFilter::All => {
                for (_, group) in &mut self.groups {
                    if let Some(type_index) = group.type_index(type_id) {
                        let component_type = group.type_mut(*type_index).unwrap();
                        let type_len = component_type.len();
                        if type_len > 0 {
                            len += type_len;
                            iters.push(component_type.iter_mut());
                        }
                    }
                }
            }
            GroupFilter::Active => {
                let key = (C::CONFIG.priority, C::IDENTIFIER);
                if let Some(cluster) = self.active_components.get(&key).cloned() {
                    return self.path_mut(&ComponentPath::new(&cluster.paths()));
                }
            }
            GroupFilter::Specific(group_ids) => {
                let mut indices: Vec<ArenaIndex> = group_ids
                    .iter()
                    .filter_map(|group_id| self.group_index(group_id).cloned())
                    .collect();
                indices.sort_by(|a, b| a.index().cmp(&b.index()));
                let mut head: &mut [ArenaEntry<_>] = self.groups.as_slice();
                let mut offset = 0;
                for index in &indices {
                    let split = head.split_at_mut(index.index() as usize + 1 - offset);
                    head = split.1;
                    offset += split.0.len();
                    match split.0.last_mut().unwrap() {
                        ArenaEntry::Occupied { data, .. } => {
                            if let Some(type_index) = data.type_index(type_id) {
                                let component_type = data.type_mut(*type_index).unwrap();
                                let type_len = component_type.len();
                                if type_len > 0 {
                                    len += type_len;
                                    iters.push(component_type.iter_mut());
                                }
                            }
                        }
                        _ => unreachable!(),
                    };
                }
            }
        };

        return ComponentSetMut::new(iters, len);
    }

    pub fn boxed_component(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        if let Some(group) = self.groups.get(handle.group_index()) {
            if let Some(component_type) = group.type_ref(handle.type_index()) {
                return component_type.component(handle.component_index());
            }
        }
        return None;
    }

    pub fn boxed_component_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
        if let Some(group) = self.groups.get_mut(handle.group_index()) {
            if let Some(component_type) = group.type_mut(handle.type_index()) {
                return component_type.component_mut(handle.component_index());
            }
        }
        return None;
    }

    pub fn component<C: ComponentDerive>(&self, handle: ComponentHandle) -> Option<&C> {
        if let Some(group) = self.groups.get(handle.group_index()) {
            if let Some(component_type) = group.type_ref(handle.type_index()) {
                if let Some(component) = component_type.component(handle.component_index()) {
                    return component.as_ref().downcast_ref();
                }
            }
        }
        return None;
    }

    pub fn component_mut<C: ComponentDerive>(&mut self, handle: ComponentHandle) -> Option<&mut C> {
        if let Some(group) = self.groups.get_mut(handle.group_index()) {
            if let Some(component_type) = group.type_mut(handle.type_index()) {
                if let Some(component) = component_type.component_mut(handle.component_index()) {
                    return component.as_mut().downcast_mut();
                }
            }
        }
        return None;
    }

    pub fn does_group_exist(&self, group: u16) -> bool {
        self.group_map.contains_key(&group)
    }

    pub(crate) fn group_mut(&mut self, index: ArenaIndex) -> Option<&mut ComponentGroup> {
        self.groups.get_mut(index)
    }

    pub(crate) fn group(&self, index: ArenaIndex) -> Option<&ComponentGroup> {
        self.groups.get(index)
    }

    pub(crate) fn group_index(&self, id: &u16) -> Option<&ArenaIndex> {
        return self.group_map.get(id);
    }

    pub fn active_group_ids(&self) -> &[u16] {
        return &self.active_group_ids;
    }

    pub fn group_ids(&self) -> impl Iterator<Item = &u16> {
        self.group_map.keys()
    }

    pub fn groups(&self) -> impl Iterator<Item = &ComponentGroup> {
        self.groups.iter().map(|(_, group)| group)
    }

    pub fn groups_mut(&mut self) -> impl Iterator<Item = &mut ComponentGroup> {
        self.groups.iter_mut().map(|(_, group)| group)
    }

    pub fn group_by_id(&self, id: u16) -> Option<&ComponentGroup> {
        if let Some(group_index) = self.group_index(&id) {
            return self.group(*group_index);
        }
        return None;
    }

    pub fn group_by_id_mut(&mut self, id: u16) -> Option<&mut ComponentGroup> {
        if let Some(group_index) = self.group_index(&id) {
            return self.group_mut(*group_index);
        }
        return None;
    }

    pub const fn render_components(&self) -> bool {
        self.render_components
    }

    pub(crate) fn component_callbacks(&self, type_id: &ComponentTypeId) -> &ComponentCallbacks {
        return self.component_callbacks.get(&type_id).unwrap();
    }

    #[cfg(feature = "serde")]
    pub(crate) fn register_callbacks<C: ComponentController>(&mut self) {
        self.component_callbacks
            .insert(C::IDENTIFIER, ComponentCallbacks::new::<C>());
    }

    pub(crate) fn copy_active_components(
        &self,
    ) -> Rc<BTreeMap<(i16, ComponentTypeId), ComponentCluster>> {
        return self.active_components.clone();
    }

    pub fn set_render_components(&mut self, render_components: bool) {
        self.render_components = render_components
    }

    pub fn group_deltas(&self) -> &[GroupDelta] {
        &self.group_deltas
    }

    #[cfg(feature = "physics")]
    pub fn world(&self) -> Ref<World> {
        self.world.borrow()
    }

    #[cfg(feature = "physics")]
    pub fn world_mut(&mut self) -> RefMut<World> {
        self.world.borrow_mut()
    }

    #[cfg(feature = "physics")]
    pub fn world_rc(&self) -> Rc<RefCell<World>> {
        self.world.clone()
    }

    pub fn instance_buffer<C: ComponentController>(
        &self,
        group_id: u16,
    ) -> Option<&InstanceBuffer> {
        if let Some(group_index) = self.group_map.get(&group_id) {
            let group = self.groups.get(*group_index).unwrap();
            if let Some(component_type_index) = group.type_index(C::IDENTIFIER) {
                let component_type = group.type_ref(*component_type_index).unwrap();
                return component_type.buffer();
            }
        }
        return None;
    }

    #[cfg(feature = "physics")]
    pub fn collision_event(
        &mut self,
    ) -> Result<rapier2d::prelude::CollisionEvent, crossbeam::channel::TryRecvError> {
        self.world.borrow_mut().collision_event()
    }
}
