#[cfg(feature = "physics")]
use crate::physics::{PhysicsComponent, World};
use crate::{
    Arena, ArenaEntry, ArenaIndex, ArenaPath, Camera, ComponentCluster, ComponentController,
    ComponentGroup, ComponentGroupDescriptor, ComponentHandle, ComponentSet, ComponentSetMut,
    DynamicComponent, Gpu, DEFAULT_GROUP_ID,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::TypeId;
use std::collections::BTreeMap;

/// Access to the component system.
pub struct ComponentManager {
    group_map: FxHashMap<u32, ArenaIndex>,
    groups: Arena<ComponentGroup>,
    active_groups: FxHashSet<ArenaIndex>,
    active_group_ids: Vec<u32>,

    update_components: bool,
    render_components: bool,

    id_counter: u32,
    remove_current_commponent: bool,
    force_update_sets: bool,
    current_component: Option<ComponentHandle>,
    active_components: BTreeMap<(i16, TypeId), ComponentCluster>,
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
            active_components: Default::default(),
        }
    }

    pub(crate) fn update_sets(&mut self, camera: &Camera) {
        let mut active_groups = vec![];
        let camera_rect = camera.rect();
        for (index, group) in &mut self.groups {
            if group.enabled() && group.intersects_camera(camera_rect.0, camera_rect.1) {
                group.set_active(true);
                active_groups.push((index, group));
            } else {
                group.set_active(false);
            }
        }

        let new_ids: FxHashSet<ArenaIndex> =
            active_groups.iter().map(|(index, _)| *index).collect();
        let mut difference = &self.active_groups - &new_ids;
        difference.extend(&new_ids - &self.active_groups);

        if self.force_update_sets || !difference.is_empty() {
            self.force_update_sets = false;
            for set in self.active_components.values_mut() {
                set.clear();
            }
            for (group_index, group) in &mut active_groups {
                for (type_index, component_type) in group.types() {
                    if component_type.is_empty() {
                        continue;
                    }
                    let type_id = *component_type.type_id();
                    let priority = component_type.config().priority;
                    let key = (priority, type_id);
                    let path = ArenaPath {
                        group_index: *group_index,
                        type_index,
                    };
                    if let Some(active_component) = self.active_components.get_mut(&key) {
                        active_component.add(path);
                    } else {
                        let config = component_type.config();
                        self.active_components
                            .insert(key, ComponentCluster::new(path, config));
                    }
                }
            }
            self.active_groups = new_ids;
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

    pub fn force_buffer<T: ComponentController>(&mut self) {
        let type_id = TypeId::of::<T>();
        for group in &mut self.groups {
            if let Some(index) = group.1.type_index(&type_id) {
                let component_type = group.1.type_mut(*index).unwrap();
                component_type.set_force_rewrite_buffer(true);
            }
        }
    }

    pub fn force_buffer_groups<T: ComponentController>(&mut self, groups: &[u32]) {
        let type_id = TypeId::of::<T>();
        for group_id in groups {
            if let Some(group_index) = self.group_map.get(group_id) {
                let group = &mut self.groups[*group_index];
                if let Some(index) = group.type_index(&type_id) {
                    let component_type = group.type_mut(*index).unwrap();
                    component_type.set_force_rewrite_buffer(true);
                }
            }
        }
    }

    pub fn force_buffer_active<T: ComponentController>(&mut self) {
        let type_id = TypeId::of::<T>();
        for group in self.active_groups.iter() {
            let group = &mut self.groups[*group];
            if let Some(index) = group.type_index(&type_id) {
                let component_type = group.type_mut(*index).unwrap();
                component_type.set_force_rewrite_buffer(true);
            }
        }
    }

    pub fn create_component<T: 'static + ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        total_frames: u64,
        group_id: Option<u32>,
        component: T,
    ) -> (&mut T, ComponentHandle) {
        let group_id = group_id.unwrap_or(DEFAULT_GROUP_ID);
        let type_id = TypeId::of::<T>();
        let config = T::config();

        let group_index = self
            .group_map
            .get(&group_id)
            .expect(format!("Group {} does not exist!", group_id).as_str());
        let group = &mut self.groups[*group_index];
        let component = Box::new(component);
        let handle;

        if let Some(type_index) = group.type_index(&type_id).copied() {
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
            let (type_index, index) = group.add_component_type(type_id, config, component);
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

    pub fn remove_components<T: ComponentController>(
        &mut self,
        group_ids: Option<&[u32]>,
        #[cfg(feature = "physics")] world: &mut World,
    ) {
        let type_id = TypeId::of::<T>();
        let group_ids = group_ids.unwrap_or(&self.active_group_ids);
        for group_id in group_ids {
            if let Some(group_index) = self.group_map.get(&group_id) {
                let group = self.groups.get_mut(*group_index).unwrap();
                if let Some(type_index) = group.type_index(&type_id) {
                    if let Some(current_handle) = &self.current_component {
                        if *group_index == current_handle.group_index()
                            && *type_index == current_handle.type_index()
                        {
                            self.remove_current_commponent = true;
                        }
                    }
                    let component_type = group.type_mut(*type_index).unwrap();
                    #[cfg(feature = "physics")]
                    for (_, c) in component_type.iter_mut() {
                        if let Some(p) = c.inner_mut().downcast_mut::<PhysicsComponent>() {
                            p.remove_from_world(world);
                        }
                    }
                    component_type.clear();
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
                for (_, component_type) in group.types() {
                    for (_, c) in component_type.iter_mut() {
                        if let Some(p) = c.inner_mut().downcast_mut::<PhysicsComponent>() {
                            p.remove_from_world(world);
                        }
                    }
                }
            }

            #[cfg(not(feature = "physics"))]
            self.groups.remove(index);
        }
    }

    pub fn components<T: 'static + ComponentController>(
        &self,
        group_ids: Option<&[u32]>,
    ) -> ComponentSet<T> {
        let type_id = TypeId::of::<T>();
        let group_ids: &[u32] = group_ids.unwrap_or(&self.active_group_ids);

        let mut types = vec![];
        let mut len = 0;
        let mut group_handles: Vec<ArenaIndex> = group_ids
            .iter()
            .filter_map(|group_id| self.group_index(group_id).copied())
            .collect();
        group_handles.sort_by(|a, b| a.index().cmp(&b.index()));
        for handle in group_handles {
            let group = self.group(handle).unwrap();
            if let Some(type_index) = group.type_index(&type_id) {
                let component_type = group.type_ref(*type_index).unwrap();
                let type_len = component_type.len();
                if type_len > 0 {
                    len += type_len;
                    types.push(component_type);
                }
            }
        }
        return ComponentSet::new(types, len);
    }

    pub fn components_mut<T: 'static + ComponentController>(
        &mut self,
        group_ids: Option<&[u32]>,
    ) -> ComponentSetMut<T> {
        let type_id = TypeId::of::<T>();
        let group_ids: &[u32] = group_ids.unwrap_or(&self.active_group_ids);
        let mut group_handles: Vec<ArenaIndex> = group_ids
            .iter()
            .filter_map(|group_id| self.group_index(group_id).copied())
            .collect();
        group_handles.sort_by(|a, b| a.index().cmp(&b.index()));

        let mut types = vec![];
        let mut len = 0;
        let mut head: &mut [ArenaEntry<_>] = self.groups.as_slice();
        let mut offset = 0;
        for handle in group_handles {
            let split = head.split_at_mut(handle.index() as usize + 1 - offset);
            head = split.1;
            offset += split.0.len();
            match split.0.last_mut().unwrap() {
                ArenaEntry::Occupied { data, .. } => {
                    if let Some(type_index) = data.type_index(&type_id) {
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

    pub fn component<T: ComponentController>(&self, handle: &ComponentHandle) -> Option<&T> {
        if let Some(group) = self.groups.get(handle.group_index()) {
            if let Some(component_type) = group.type_ref(handle.type_index()) {
                if let Some(component) = component_type.component(handle.component_index()) {
                    return component.as_ref().downcast_ref();
                }
            }
        }
        return None;
    }

    pub fn component_mut<T: ComponentController>(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut T> {
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
    pub(crate) fn copy_active_components(&self) -> Vec<ComponentCluster> {
        return self.active_components.values().map(|c| c.clone()).collect();
    }

    #[inline]
    pub(crate) fn active_components(&self) -> &BTreeMap<(i16, TypeId), ComponentCluster> {
        return &self.active_components;
    }

    #[inline]
    pub(crate) fn remove_current_commponent(&mut self) -> bool {
        let result = self.remove_current_commponent;
        self.remove_current_commponent = false;
        return result;
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
