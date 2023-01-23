#[cfg(feature = "physics")]
use crate::physics::{PhysicsComponent, World};
use crate::{
    Arena, ArenaEntry, ArenaIndex, ArenaPath, Camera, ComponentCluster, ComponentController,
    ComponentGroup, ComponentGroupDescriptor, ComponentHandle, ComponentSet, ComponentSetMut,
    DynamicComponent, Gpu, DEFAULT_GROUP_ID,
};
use log::info;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub enum GroupFilter<'a> {
    All,
    Active,
    Specific(&'a [u32]),
}

impl<'a> Default for GroupFilter<'a> {
    fn default() -> Self {
        return GroupFilter::Active;
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
// #[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
/// Access to the component system.
pub struct ComponentManager {
    update_components: bool,
    render_components: bool,

    id_counter: u32,
    remove_current_commponent: bool,
    force_update_sets: bool,
    current_component: Option<ComponentHandle>,
    group_map: FxHashMap<u32, ArenaIndex>,
    #[serde(bound(deserialize = "ComponentGroup: serde::Deserialize<'de>"))]
    groups: Arena<ComponentGroup>,

    #[serde(skip)]
    #[serde(default)]
    active_groups: FxHashSet<ArenaIndex>,

    #[serde(skip)]
    #[serde(default)]
    active_group_ids: Vec<u32>,

    #[serde(skip)]
    #[serde(default)]
    active_components: Option<BTreeMap<(i16, &'static str), ComponentCluster>>,
}

impl ComponentManager {
    pub(crate) fn new() -> Self {
        let default_component_group = ComponentGroup::default();
        let mut groups = Arena::default();
        let mut group_map = FxHashMap::default();
        let index = groups.insert(default_component_group);
        group_map.insert(DEFAULT_GROUP_ID, index);
        Self {
            active_groups: FxHashSet::from_iter([index]),
            active_group_ids: vec![DEFAULT_GROUP_ID],
            groups,
            group_map,

            update_components: true,
            render_components: true,

            id_counter: 0,
            remove_current_commponent: false,
            force_update_sets: false,
            current_component: Default::default(),
            active_components: Some(Default::default()),
        }
    }

    pub(crate) fn update_sets(&mut self, camera: &Camera) {
        let camera_rect = camera.rect();
        let active_components = self.active_components.as_mut().unwrap();
        let mut groups_changed = false;
        for (index, group) in &mut self.groups {
            if group.enabled() && group.intersects_camera(camera_rect.0, camera_rect.1) {
                group.set_active(true);
                if self.active_groups.insert(index) {
                    groups_changed = true;
                }
            } else {
                group.set_active(false);
                if !self.active_groups.remove(&index) {
                    groups_changed = true;
                }
            }
        }

        if self.force_update_sets || groups_changed {
            info!("Rebuilding Active Components...");
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
                    let name = component_type.name();
                    let priority = component_type.config().priority;
                    let key = (priority, name);
                    let path = ArenaPath {
                        group_index: *index,
                        type_index,
                    };
                    if let Some(active_component) = active_components.get_mut(&key) {
                        active_component.add(path);
                    } else {
                        let config = component_type.config();
                        active_components.insert(key, ComponentCluster::new(path, config.clone()));
                    }
                }
            }
            self.active_group_ids = self
                .active_groups
                .iter()
                .map(|i| self.groups[*i].id())
                .collect();
        }
    }

    pub(crate) fn buffer_sets(&mut self, gpu: &Gpu, #[cfg(feature = "physics")] world: &World) {
        for group in &self.active_groups {
            if let Some(group) = self.groups.get_mut(*group) {
                for (_, t) in group.types() {
                    t.buffer_data(
                        gpu,
                        #[cfg(feature = "physics")]
                        world,
                    );
                }
            }
        }
    }

    pub fn force_buffer<C: ComponentController>(&mut self) {
        let name = C::name();
        for group in &mut self.groups {
            if let Some(index) = group.1.type_index(name) {
                let component_type = group.1.type_mut(*index).unwrap();
                component_type.set_force_rewrite_buffer(true);
            }
        }
    }

    pub fn force_buffer_groups<C: ComponentController>(&mut self, groups: &[u32]) {
        let name = C::name();
        for group_id in groups {
            if let Some(group_index) = self.group_map.get(group_id) {
                let group = &mut self.groups[*group_index];
                if let Some(index) = group.type_index(name) {
                    let component_type = group.type_mut(*index).unwrap();
                    component_type.set_force_rewrite_buffer(true);
                }
            }
        }
    }

    pub fn force_buffer_active<C: ComponentController>(&mut self) {
        let name = C::name();
        for group in self.active_groups.iter() {
            let group = &mut self.groups[*group];
            if let Some(index) = group.type_index(name) {
                let component_type = group.type_mut(*index).unwrap();
                component_type.set_force_rewrite_buffer(true);
            }
        }
    }

    pub fn create_component<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        total_frames: u64,
        group_id: Option<u32>,
        component: C,
    ) -> (&mut C, ComponentHandle) {
        let group_id = group_id.unwrap_or(DEFAULT_GROUP_ID);
        let name = C::name();
        let group_index = self
            .group_map
            .get(&group_id)
            .expect(format!("Group {} does not exist!", group_id).as_str());
        let group = &mut self.groups[*group_index];
        let handle;

        if let Some(type_index) = group.type_index(name).copied() {
            self.id_counter += 1;
            let component_type = group.type_mut(type_index).unwrap();
            let index = component_type.add(component);
            handle = ComponentHandle::new(
                index,
                type_index,
                *group_index,
                total_frames,
                self.id_counter,
            );
        } else {
            // Create a new ComponentType
            self.id_counter += 1;
            self.force_update_sets = true;
            let (type_index, index) = group.add_component_type(component);
            handle = ComponentHandle::new(
                index,
                type_index,
                *group_index,
                total_frames,
                self.id_counter,
            );
        }

        let c = self.component_dynamic_mut(&handle).unwrap();
        c.inner_mut().init(
            #[cfg(feature = "physics")]
            world,
            handle,
        );
        return (c.downcast_mut().unwrap(), handle);
    }

    pub fn create_group(&mut self, descriptor: &ComponentGroupDescriptor) {
        let group = ComponentGroup::new(descriptor.id, descriptor);
        let index = self.groups.insert(group);
        self.group_map.insert(descriptor.id, index);
    }

    pub fn remove_component(
        &mut self,
        handle: &ComponentHandle,
        #[cfg(feature = "physics")] world: &mut World,
    ) -> Option<DynamicComponent> {
        if let Some(group) = self.groups.get_mut(handle.group_index()) {
            if let Some(current_handle) = &self.current_component {
                if handle == current_handle {
                    self.remove_current_commponent = true;
                    return None;
                }
            }
            if let Some(component_type) = group.type_mut(handle.type_index()) {
                #[cfg(feature = "physics")]
                if let Some(mut component) = component_type.remove(handle) {
                    if let Some(p) = component.inner_mut().downcast_mut::<PhysicsComponent>() {
                        p.remove_from_world(world);
                    }
                    return Some(component);
                }
                #[cfg(not(feature = "physics"))]
                return component_type.remove(handle);
            }
        }
        return None;
    }

    #[inline]
    pub fn remove_components<C: ComponentController>(
        &mut self,
        group_filter: &GroupFilter,
        #[cfg(feature = "physics")] world: &mut World,
    ) {
        #[inline]
        fn remove(
            current: &mut Option<ComponentHandle>,
            remove_current: &mut bool,
            group: &mut ComponentGroup,
            group_index: ArenaIndex,
            name: &'static str,
            #[cfg(feature = "physics")] world: &mut World,
        ) {
            if let Some(type_index) = group.type_index(name) {
                if let Some(current_handle) = current {
                    if group_index == current_handle.group_index()
                        && *type_index == current_handle.type_index()
                    {
                        *remove_current = true;
                    }
                }
                let component_type = group.type_mut(*type_index).unwrap();
                #[cfg(feature = "physics")]
                for (_, c) in component_type.iter_mut() {
                    if let Some(p) = c.inner_mut().downcast_mut::<PhysicsComponent>() {
                        p.remove_from_world(world);
                    } else {
                        break;
                    }
                }
                component_type.clear();
            }
        }

        let name = C::name();
        match group_filter {
            GroupFilter::All => {
                for (index, group) in &mut self.groups {
                    remove(
                        &mut self.current_component,
                        &mut self.remove_current_commponent,
                        group,
                        index,
                        name,
                        #[cfg(feature = "physics")]
                        world,
                    );
                }
            }
            GroupFilter::Active => {
                for index in &self.active_groups {
                    if let Some(group) = self.groups.get_mut(*index) {
                        remove(
                            &mut self.current_component,
                            &mut self.remove_current_commponent,
                            group,
                            *index,
                            name,
                            #[cfg(feature = "physics")]
                            world,
                        )
                    }
                }
            }
            GroupFilter::Specific(group_ids) => {
                for group_id in *group_ids {
                    if let Some(index) = self.group_map.get(&group_id) {
                        let group = self.groups.get_mut(*index).unwrap();
                        remove(
                            &mut self.current_component,
                            &mut self.remove_current_commponent,
                            group,
                            *index,
                            name,
                            #[cfg(feature = "physics")]
                            world,
                        )
                    }
                }
            }
        }
    }

    pub fn remove_group(&mut self, group_id: u32, #[cfg(feature = "physics")] world: &mut World) {
        if group_id == DEFAULT_GROUP_ID {
            panic!("Cannot the default group with ID {DEFAULT_GROUP_ID}!");
        }

        if let Some(index) = self.group_map.remove(&group_id) {
            #[cfg(feature = "physics")] // TODO: Find a way to fix iterating over all components
            if let Some(mut group) = self.groups.remove(index) {
                'outer: for (_, component_type) in group.types() {
                    for (_, c) in component_type.iter_mut() {
                        if let Some(p) = c.inner_mut().downcast_mut::<PhysicsComponent>() {
                            p.remove_from_world(world);
                        } else {
                            break 'outer;
                        }
                    }
                }
            }

            #[cfg(not(feature = "physics"))]
            self.groups.remove(index);
        }
    }

    pub fn components<C: ComponentController>(
        &self,
        group_filter: &GroupFilter,
    ) -> ComponentSet<C> {
        let name = C::name();
        let mut types = vec![];
        let mut len = 0;

        match group_filter {
            GroupFilter::All => {
                for (_, group) in &self.groups {
                    if let Some(type_index) = group.type_index(name) {
                        let component_type = group.type_ref(*type_index).unwrap();
                        let type_len = component_type.len();
                        if type_len > 0 {
                            len += type_len;
                            types.push(component_type);
                        }
                    }
                }
            }
            GroupFilter::Active => {
                for index in &self.active_groups {
                    if let Some(group) = self.groups.get(*index) {
                        if let Some(type_index) = group.type_index(name) {
                            let component_type = group.type_ref(*type_index).unwrap();
                            let type_len = component_type.len();
                            if type_len > 0 {
                                len += type_len;
                                types.push(component_type);
                            }
                        };
                    }
                }
            }
            GroupFilter::Specific(group_ids) => {
                for group_id in *group_ids {
                    if let Some(index) = self.group_map.get(&group_id) {
                        let group = self.groups.get(*index).unwrap();

                        if let Some(type_index) = group.type_index(name) {
                            let component_type = group.type_ref(*type_index).unwrap();
                            let type_len = component_type.len();
                            if type_len > 0 {
                                len += type_len;
                                types.push(component_type);
                            }
                        };
                    }
                }
            }
        };

        return ComponentSet::new(types, len);
    }

    pub fn components_mut<C: ComponentController>(
        &mut self,
        group_filter: &GroupFilter,
    ) -> ComponentSetMut<C> {
        let name = C::name();
        let mut types = vec![];
        let mut len = 0;

        match group_filter {
            GroupFilter::All => {
                for (_, group) in &mut self.groups {
                    if let Some(type_index) = group.type_index(name) {
                        let component_type = group.type_mut(*type_index).unwrap();
                        let type_len = component_type.len();
                        if type_len > 0 {
                            len += type_len;
                            types.push(component_type);
                        }
                    }
                }
            }
            GroupFilter::Active => {
                let mut indices: Vec<ArenaIndex> = self.active_groups.iter().map(|i| *i).collect();
                indices.sort_by(|a, b| a.index().cmp(&b.index()));
                let mut head: &mut [ArenaEntry<_>] = self.groups.as_slice();
                let mut offset = 0;
                for index in &indices {
                    let split = head.split_at_mut(index.index() as usize + 1 - offset);
                    head = split.1;
                    offset += split.0.len();
                    match split.0.last_mut().unwrap() {
                        ArenaEntry::Occupied { data, .. } => {
                            if let Some(type_index) = data.type_index(name) {
                                let component_type = data.type_mut(*type_index).unwrap();
                                let type_len = component_type.len();
                                if type_len > 0 {
                                    len += type_len;
                                    types.push(component_type);
                                }
                            }
                        }
                        _ => unreachable!(),
                    };
                }
            }
            GroupFilter::Specific(group_ids) => {
                let mut indices: Vec<ArenaIndex> = group_ids
                    .iter()
                    .filter_map(|group_id| self.group_index(group_id).copied())
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
                            if let Some(type_index) = data.type_index(name) {
                                let component_type = data.type_mut(*type_index).unwrap();
                                let type_len = component_type.len();
                                if type_len > 0 {
                                    len += type_len;
                                    types.push(component_type);
                                }
                            }
                        }
                        _ => unreachable!(),
                    };
                }
            }
        };

        return ComponentSetMut::new(types, len);
    }

    pub fn component_dynamic(&self, handle: &ComponentHandle) -> Option<&DynamicComponent> {
        if let Some(group) = self.groups.get(handle.group_index()) {
            if let Some(component_type) = group.type_ref(handle.type_index()) {
                return component_type.component(handle.component_index());
            }
        }
        return None;
    }

    pub fn component_dynamic_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut DynamicComponent> {
        if let Some(group) = self.groups.get_mut(handle.group_index()) {
            if let Some(component_type) = group.type_mut(handle.type_index()) {
                return component_type.component_mut(handle.component_index());
            }
        }
        return None;
    }

    pub fn component<C: ComponentController>(&self, handle: &ComponentHandle) -> Option<&C> {
        if let Some(group) = self.groups.get(handle.group_index()) {
            if let Some(component_type) = group.type_ref(handle.type_index()) {
                if let Some(component) = component_type.component(handle.component_index()) {
                    return component.as_ref().downcast_ref();
                }
            }
        }
        return None;
    }

    pub fn component_mut<C: ComponentController>(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut C> {
        if let Some(group) = self.groups.get_mut(handle.group_index()) {
            if let Some(component_type) = group.type_mut(handle.type_index()) {
                if let Some(component) = component_type.component_mut(handle.component_index()) {
                    return component.as_mut().downcast_mut();
                }
            }
        }
        return None;
    }

    pub fn does_group_exist(&self, group: u32) -> bool {
        self.group_map.contains_key(&group)
    }

    // Getters
    #[inline]
    pub(crate) fn group_mut(&mut self, index: ArenaIndex) -> Option<&mut ComponentGroup> {
        self.groups.get_mut(index)
    }

    #[inline]
    pub(crate) fn group(&self, index: ArenaIndex) -> Option<&ComponentGroup> {
        self.groups.get(index)
    }

    #[inline]
    pub(crate) fn group_index(&self, id: &u32) -> Option<&ArenaIndex> {
        return self.group_map.get(id);
    }

    #[inline]
    pub(crate) fn borrow_component(
        &mut self,
        path: ArenaPath,
        index: usize,
    ) -> Option<ArenaEntry<DynamicComponent>> {
        if let Some(group) = self.groups.get_mut(path.group_index) {
            if let Some(component_type) = group.type_mut(path.type_index) {
                return component_type.borrow_component(index);
            }
        }
        return None;
    }

    #[inline]
    pub(crate) fn return_component(
        &mut self,
        path: ArenaPath,
        index: usize,
        component: ArenaEntry<DynamicComponent>,
    ) {
        if let Some(group) = self.groups.get_mut(path.group_index) {
            if let Some(component_type) = group.type_mut(path.type_index) {
                component_type.return_component(index, component);
            }
        }
    }

    #[inline]
    pub(crate) fn not_return_component(&mut self, path: ArenaPath, index: usize) {
        if let Some(group) = self.groups.get_mut(path.group_index) {
            if let Some(component_type) = group.type_mut(path.type_index) {
                component_type.not_return_component(index);
            }
        }
    }

    #[inline]
    pub fn active_group_ids(&self) -> &[u32] {
        return &self.active_group_ids;
    }

    #[inline]
    pub fn group_ids(&self) -> Vec<u32> {
        self.groups.iter().map(|(_, group)| group.id()).collect()
    }

    #[inline]
    pub const fn update_components(&self) -> bool {
        self.update_components
    }

    #[inline]
    pub const fn render_components(&self) -> bool {
        self.render_components
    }

    #[inline]
    pub(crate) fn active_components(&self) -> &BTreeMap<(i16, &'static str), ComponentCluster> {
        return self.active_components.as_ref().unwrap();
    }

    #[inline]
    pub(crate) fn borrow_active_components(
        &mut self,
    ) -> BTreeMap<(i16, &'static str), ComponentCluster> {
        return self.active_components.take().unwrap();
    }

    #[inline]
    pub(crate) fn return_active_components(
        &mut self,
        active_components: BTreeMap<(i16, &'static str), ComponentCluster>,
    ) {
        return self.active_components = Some(active_components);
    }

    #[inline]
    pub(crate) fn remove_current_commponent(&mut self) -> bool {
        let result = self.remove_current_commponent;
        self.remove_current_commponent = false;
        return result;
    }

    #[inline]
    pub(crate) fn current_component(&self) -> Option<ComponentHandle> {
        self.current_component.clone()
    }

    // Setters
    #[inline]
    pub(crate) fn set_current_component(&mut self, current_component: Option<ComponentHandle>) {
        self.current_component = current_component
    }

    #[inline]
    pub fn set_update_components(&mut self, update_components: bool) {
        self.update_components = update_components
    }

    #[inline]
    pub fn set_render_components(&mut self, render_components: bool) {
        self.render_components = render_components
    }
}
