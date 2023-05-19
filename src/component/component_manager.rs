#[cfg(feature = "physics")]
use crate::physics::{RcWorld, World};
use crate::{
    ActiveComponents, Arena, ArenaEntry, ArenaIndex, ArenaPath, BoxedComponent, CameraBuffer,
    ComponentCallbacks, ComponentController, ComponentDerive, ComponentGroup, ComponentHandle,
    ComponentSet, ComponentSetMut, ComponentType, ComponentTypeId, Gpu, GpuDefaults,
    GroupActivation, GroupHandle, InstanceBuffer, TypeIndex, Vector, CallableType,
};
use instant::Instant;
#[cfg(feature = "log")]
use log::info;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::rc::Rc;

// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
// /// Changed [Groups](crate::ComponentGroup) between frames
// pub enum GroupDelta {
//     Add(ComponentGroupId),
//     Remove(ComponentGroupId),
// }

const NO_TYPE_ERROR: &'static str = "Type first needs to be registered with .register";

macro_rules! group_filter {
    ($self:ident, $filter: ident) => {
        match $filter {
            ComponentFilter::All => &$self.all_groups,
            ComponentFilter::Active => &$self.active_groups,
            ComponentFilter::Specific(h) => h,
        }
    };
}

macro_rules! type_ref {
    ($self:ident, $C: ident) => {{
        let key = ($C::CONFIG.priority, $C::IDENTIFIER);
        let idx = $self
            .type_map
            .get(&key)
            .expect(NO_TYPE_ERROR);
        let ty = $self.types.get(idx.0).unwrap();
        ty
    }};
}

macro_rules! type_mut {
    ($self:ident, $C: ident) => {{
        let key = ($C::CONFIG.priority, $C::IDENTIFIER);
        let idx = $self
            .type_map
            .get(&key)
            .expect(NO_TYPE_ERROR);
        let ty = $self.types.get_mut(idx.0).unwrap();
        ty
    }};
}

macro_rules! type2_mut {
    ($self:ident, $C1: ident, $C2: ident) => {{
        let key1 = ($C1::CONFIG.priority, $C1::IDENTIFIER);
        let idx1 = $self
            .type_map
            .get(&key1)
            .expect(NO_TYPE_ERROR);
        let key2 = ($C2::CONFIG.priority, $C2::IDENTIFIER);
        let idx2 = $self
            .type_map
            .get(&key2)
            .expect(NO_TYPE_ERROR);
        let (ty1, ty2) = $self.types.get2_mut(idx1.0, idx2.0);
        (ty1.unwrap(), ty2.unwrap())
    }};
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
/// Filter to query which components should be in a [ComponentSet]
pub enum ComponentFilter<'a> {
    All,
    Active,
    Specific(&'a [GroupHandle]),
}

impl<'a> Default for ComponentFilter<'a> {
    fn default() -> Self {
        return ComponentFilter::DEFAULT_GROUP;
    }
}

impl ComponentFilter<'static> {
    pub const DEFAULT_GROUP: Self = ComponentFilter::Specific(&[GroupHandle::DEFAULT_GROUP]);
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Access to the component system.
pub struct ComponentManager {
    type_map: FxHashMap<(i16, ComponentTypeId), TypeIndex>,
    update_callable_types: bool,
    callable_types: Rc<Vec<CallableType>>,
    types: Arena<ComponentType>,
    groups: Arena<ComponentGroup>,
    active_groups: Vec<GroupHandle>,
    all_groups: Vec<GroupHandle>,
}

impl ComponentManager {
    pub(crate) fn new() -> Self {
        let default_component_group = ComponentGroup::new(GroupActivation::Always, 0);
        let mut groups = Arena::default();
        let index = groups.insert(default_component_group);
        Self {
            types: Default::default(),
            type_map: Default::default(),
            all_groups: Default::default(),
            callable_types: Default::default(),
            update_callable_types: true,
            active_groups: Vec::from_iter([GroupHandle(index)]),
            groups,
        }
    }

    pub(crate) fn update_sets(&mut self, camera: &CameraBuffer) {
        let cam_aabb = camera.model().aabb(Vector::new(0.0, 0.0).into()); // Translation is already applied
        let now = Instant::now();
        self.active_groups.clear();
        for (index, group) in &mut self.groups {
            if group.intersects_camera(cam_aabb) {
                group.set_active(true);
                self.active_groups.push(GroupHandle(index));
            }
        }
    }

    pub(crate) fn buffer(&mut self, gpu: &Gpu) {
        for (_, ty) in &mut self.types {
            ty.buffer(&self.active_groups, gpu);
        }
    }

    pub(crate) fn callable_types(&mut self) -> Rc<Vec<CallableType>> {
        if self.update_callable_types {
            let callables = Rc::get_mut(&mut self.callable_types).unwrap();
            callables.clear();
            for (_, ty) in &self.types {
                callables.push(CallableType::new(ty));
            }
        }
        return self.callable_types.clone();
    }

    pub fn register<C: ComponentController>(&mut self) {
        let key = (C::CONFIG.priority, C::IDENTIFIER);
        if !self.type_map.contains_key(&key) {
            let index = self
                .types
                .insert_with(|idx| ComponentType::new::<C>(TypeIndex(idx), &self.groups));
            self.type_map.insert(key, TypeIndex(index));
            self.update_callable_types = true;
        }
    }

    pub fn active_groups(&self) -> &[GroupHandle] {
        &self.active_groups
    }

    pub fn all_groups(&self) -> &[GroupHandle] {
        &self.all_groups
    }

    pub fn groups(&self) -> impl Iterator<Item = (GroupHandle, &ComponentGroup)> {
        return self
            .groups
            .iter()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn groups_mut(&mut self) -> impl Iterator<Item = (GroupHandle, &mut ComponentGroup)> {
        return self
            .groups
            .iter_mut()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn contains_group(&self, handle: GroupHandle) -> bool {
        return self.groups.contains(handle.0);
    }

    pub fn group(&self, handle: GroupHandle) -> Option<&ComponentGroup> {
        return self.groups.get(handle.0);
    }

    pub fn group_mut(&mut self, handle: GroupHandle) -> Option<&mut ComponentGroup> {
        return self.groups.get_mut(handle.0);
    }

    pub fn add_group(&mut self, group: ComponentGroup) -> GroupHandle {
        let handle = GroupHandle(self.groups.insert(group));
        for (_, ty) in &mut self.types {
            let result = ty.add_group();
            assert_eq!(handle, result);
        }
        self.all_groups.push(handle);
        return handle;
    }

    pub fn remove_group(&mut self, handle: GroupHandle) -> Option<ComponentGroup> {
        let group = self.groups.remove(handle.0);
        for (_, ty) in &mut self.types {
            ty.remove_group(handle);
        }
        self.all_groups.retain(|h| *h != handle);
        return group;
    }

    pub fn get_many<'a, C: ComponentController>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSet<'a, C> {
        let groups = group_filter!(self, filter);
        let ty = type_ref!(self, C);
        return ComponentSet::new(ty, groups);
    }
    pub fn get_many_mut<'a, C: ComponentController>(
        &mut self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let groups = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        return ComponentSetMut::new(ty, groups);
    }
    pub fn get2_many_mut<'a, C1: ComponentController, C2: ComponentController>(
        &mut self,
        filter1: ComponentFilter<'a>,
        filter2: ComponentFilter<'a>,
    ) -> (ComponentSetMut<'a, C1>, ComponentSetMut<'a, C2>) {
        assert_ne!(C1::IDENTIFIER, C2::IDENTIFIER);
        let groups1 = group_filter!(self, filter1);
        let groups2 = group_filter!(self, filter2);
        let (ty1, ty2) = type2_mut!(self, C1, C2);
        return (ComponentSetMut::<C1>::new(ty1, groups1), ComponentSetMut::<C2>::new(ty2, groups2))
    }

    // pub fn remove_components(&mut self) {}
    pub fn retain<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
        keep: impl FnMut(&mut C) -> bool,
    ) {
        let groups = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        ty.retain(groups, keep);
    }
    pub fn index<C: ComponentController>(&self, group: GroupHandle, index: usize) -> Option<&C> {
        let ty = type_mut!(self, C);
        ty.index(group, index)
    }
    pub fn index_mut<C: ComponentController>(
        &mut self,
        group: GroupHandle,
        index: usize,
    ) -> Option<&mut C> {
        let ty = type_mut!(self, C);
        ty.index_mut(group, index)
    }
    pub fn get<C: ComponentController>(&self, handle: ComponentHandle) -> Option<&C> {
        self.types.get(handle.type_index().0).unwrap().get(handle)
    }
    pub fn get_mut<C: ComponentController>(&mut self, handle: ComponentHandle) -> Option<&mut C> {
        self.types.get_mut(handle.type_index().0).unwrap().get_mut(handle)
    }
    pub fn get2_mut<C1: ComponentController, C2: ComponentController>(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut C1>, Option<&mut C2>) {
        assert_ne!(handle1, handle2);
        if handle1.type_index() == handle2.type_index() {
            let ty = type_mut!(self, C1);
            return ty.get2_mut::<C1, C2>(handle1, handle2);
        } else {
            let (ty1, ty2) = type2_mut!(self, C1, C2);
            return (ty1.get_mut::<C1>(handle1), ty2.get_mut::<C2>(handle2));
        }
    }
    pub fn get_boxed(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        self.types.get(handle.type_index().0).unwrap().get_boxed(handle)

    }
    pub fn get_boxed_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
        self.types.get_mut(handle.type_index().0).unwrap().get_boxed_mut(handle)
    }
    pub fn remove_component<C: ComponentController>(
        &mut self,
        handle: ComponentHandle,
    ) -> Option<Box<C>> {
        self.types.get_mut(handle.type_index().0).unwrap().remove_component(handle)
    }
    pub fn add_component<C: ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.add_component(group_handle, component)
    }
    pub fn add_components<I, C: ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        components: impl Iterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let ty = type_mut!(self, C);
        ty.add_components::<I, C>(group_handle, components)
    }
    pub fn force_buffer<C: ComponentController>(&mut self, filter: ComponentFilter) {
        let groups = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        ty.force_buffer(groups)
    }
    pub fn len<C: ComponentController>(&self, filter: ComponentFilter) -> usize {
        let groups = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        ty.len(groups)
    }

    // pub(crate) fn update_sets(&mut self, camera: &CameraBuffer) {
    //     let aabb = camera.model().aabb(Vector::new(0.0, 0.0).into());
    //     let active_components = Rc::get_mut(&mut self.active_components).unwrap();
    //     let now = Instant::now();
    //     let mut groups_changed = false;
    //     self.group_deltas.clear();
    //     for (index, group) in &mut self.groups {
    //         if group.intersects_camera(aabb.0, aabb.1) {
    //             group.set_active(true);
    //             if self.active_groups.insert(index) {
    //                 self.group_deltas.push(GroupDelta::Add(group.id()));
    //                 groups_changed = true;
    //             }
    //         } else {
    //             group.set_active(false);
    //             if self.active_groups.remove(&index) {
    //                 self.group_deltas.push(GroupDelta::Remove(group.id()));
    //                 groups_changed = true;
    //             }
    //         }
    //     }

    //     if self.force_update_sets || groups_changed {
    //         #[cfg(feature = "log")]
    //         {
    //             info!("Rebuilding Active Components");
    //             info!("Now processing {} group(s)", self.active_groups.len());
    //         }
    //         self.force_update_sets = false;
    //         for set in active_components.values_mut() {
    //             set.clear();
    //         }
    //         for index in &self.active_groups {
    //             let group = self.groups.get_mut(*index).unwrap();
    //             for (type_index, component_type) in group.types() {
    //                 if component_type.is_empty() {
    //                     continue;
    //                 }
    //                 let type_id = component_type.type_id();
    //                 let priority = component_type.config().priority;
    //                 let key = (priority, type_id);
    //                 let path = ArenaPath {
    //                     group_index: *index,
    //                     type_index,
    //                 };
    //                 if let Some(active_component) = active_components.get_mut(&key) {
    //                     active_component.add(path);
    //                     active_component.update_time(now);
    //                 } else {
    //                     let config = component_type.config();
    //                     active_components.insert(
    //                         key,
    //                         ComponentCluster::new(
    //                             path,
    //                             self.component_callbacks.get(&type_id).unwrap().clone(),
    //                             config.clone(),
    //                             now,
    //                         ),
    //                     );
    //                 }
    //             }
    //         }
    //         for cluster in active_components.values_mut() {
    //             cluster.sort(); // Sorting needed for components_mut
    //         }
    //         self.active_group_ids = self
    //             .active_groups
    //             .iter()
    //             .map(|i| self.groups[*i].id())
    //             .collect();
    //     }
    // }

    // pub(crate) fn buffer_sets(&mut self, gpu: &Gpu) {
    //     for group in &self.active_groups {
    //         if let Some(group) = self.groups.get_mut(*group) {
    //             for (_, t) in group.types() {
    //                 t.buffer_data(gpu);
    //             }
    //         }
    //     }
    // }

    // pub fn force_buffer<C: ComponentController>(&mut self, filter: ComponentFilter) {
    //     let type_id = C::IDENTIFIER;
    //     match filter {
    //         ComponentFilter::All => {
    //             for group in &mut self.groups {
    //                 if let Some(component_type) = group.1.type_by_id_mut(type_id) {
    //                     component_type.set_force_buffer(true);
    //                 }
    //             }
    //         }
    //         ComponentFilter::Active => {
    //             for group in self.active_groups.iter() {
    //                 if let Some(group) = self.groups.get_mut(*group) {
    //                     if let Some(component_type) = group.type_by_id_mut(type_id) {
    //                         component_type.set_force_buffer(true);
    //                     }
    //                 }
    //             }
    //         }
    //         ComponentFilter::Specific(groups) => {
    //             for group_id in groups {
    //                 if let Some(group_index) = self.group_map.get(group_id) {
    //                     let group = &mut self.groups[*group_index];
    //                     if let Some(component_type) = group.type_by_id_mut(type_id) {
    //                         component_type.set_force_buffer(true);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // pub fn add_component_to_group<C: ComponentController>(
    //     &mut self,
    //     group_id: ComponentGroupId,
    //     component: C,
    // ) -> ComponentHandle {
    //     return self.add_components_to_group(group_id, std::iter::once(component))[0];
    // }

    // pub fn add_component<C: ComponentController>(&mut self, component: C) -> ComponentHandle {
    //     return self.add_component_to_group(ComponentGroupId::DEFAULT, component);
    // }

    // pub fn add_components<I, C: ComponentController>(
    //     &mut self,
    //     components: I,
    // ) -> Vec<ComponentHandle>
    // where
    //     I: IntoIterator,
    //     I::IntoIter: ExactSizeIterator<Item = C>,
    // {
    //     return self.add_components_to_group(ComponentGroupId::DEFAULT, components);
    // }

    // pub fn add_components_to_group<I, C: ComponentController>(
    //     &mut self,
    //     group_id: ComponentGroupId,
    //     components: I,
    // ) -> Vec<ComponentHandle>
    // where
    //     I: IntoIterator,
    //     I::IntoIter: ExactSizeIterator<Item = C>,
    // {
    //     let type_id = C::IDENTIFIER;
    //     let group_index = self
    //         .group_map
    //         .get(&group_id)
    //         .expect(format!("Group {} does not exist!", group_id.id).as_str());
    //     let group = &mut self.groups[*group_index];

    //     self.component_callbacks
    //         .entry(type_id)
    //         .or_insert_with(|| ComponentCallbacks::new::<C>());

    //     let (type_index, component_type) = if let Some(type_index) = group.type_index(type_id) {
    //         (*type_index, group.type_mut(*type_index).unwrap())
    //     } else {
    //         self.force_update_sets = true;
    //         group.add_component_type::<C>()
    //     };

    //     let iter = components.into_iter();
    //     let mut handles = Vec::with_capacity(iter.len());
    //     for component in iter {
    //         self.id_counter += 1;
    //         let incomplete_handle = ComponentHandle::new(
    //             ArenaIndex::INVALID,
    //             type_index,
    //             *group_index,
    //             self.id_counter,
    //             group_id,
    //         );
    //         let handle = component_type.add(
    //             incomplete_handle,
    //             #[cfg(feature = "physics")]
    //             self.world.clone(),
    //             component,
    //         );
    //         handles.push(handle);
    //     }

    //     return handles;
    // }

    // pub fn add_group(&mut self, group: impl Into<ComponentGroup>) {
    //     #[allow(unused_mut)]
    //     let mut group = group.into();
    //     let group_id = group.id();
    //     assert_ne!(group_id.id, 0);
    //     assert!(self.group_map.contains_key(&group_id) == false);
    //     #[cfg(feature = "physics")]
    //     for (_, component_type) in group.types() {
    //         let type_id = component_type.type_id();
    //         for (_, component) in component_type {
    //             component
    //                 .base_mut()
    //                 .add_to_world(type_id, self.world.clone())
    //         }
    //     }
    //     let index = self.groups.insert(group);
    //     self.force_update_sets = true;
    //     self.group_map.insert(group_id, index);
    // }

    // pub fn remove_component(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
    //     if let Some(group) = self.groups.get_mut(handle.group_index()) {
    //         let type_index = handle.type_index();
    //         if let Some(component_type) = group.type_mut(type_index) {
    //             if let Some(mut to_remove) = component_type.remove(handle) {
    //                 to_remove.base_mut().deinit();
    //                 if component_type.len() == 0 {
    //                     self.force_update_sets = false;
    //                     group.remove_type(type_index);
    //                 }
    //                 return Some(to_remove);
    //             }
    //         }
    //     }
    //     return None;
    // }

    // pub fn remove_components<C: ComponentController>(&mut self, filter: ComponentFilter) {
    //     let type_id = C::IDENTIFIER;

    //     fn remove(rewrite: &mut bool, group: &mut ComponentGroup, type_id: ComponentTypeId) {
    //         if let Some(type_index) = group.type_index(type_id).cloned() {
    //             let component_type = group.type_mut(type_index).unwrap();
    //             for (_, c) in component_type.iter_mut() {
    //                 c.base_mut().deinit();
    //             }
    //             if component_type.len() == 0 {
    //                 group.remove_type(type_index);
    //                 *rewrite = true;
    //             }

    //             group.remove_type(type_index)
    //         }
    //     }

    //     match filter {
    //         ComponentFilter::All => {
    //             for (_index, group) in &mut self.groups {
    //                 remove(&mut self.force_update_sets, group, type_id)
    //             }
    //         }
    //         ComponentFilter::Active => {
    //             for index in &self.active_groups {
    //                 if let Some(group) = self.groups.get_mut(*index) {
    //                     remove(&mut self.force_update_sets, group, type_id)
    //                 }
    //             }
    //         }
    //         ComponentFilter::Specific(group_ids) => {
    //             for group_id in group_ids {
    //                 if let Some(index) = self.group_map.get(&group_id) {
    //                     let group = self.groups.get_mut(*index).unwrap();
    //                     remove(&mut self.force_update_sets, group, type_id)
    //                 }
    //             }
    //         }
    //     }
    // }

    // pub fn remove_group(&mut self, group_id: ComponentGroupId) -> Option<ComponentGroup> {
    //     if group_id == ComponentGroupId::DEFAULT {
    //         return None;
    //     }

    //     if let Some(index) = self.group_map.remove(&group_id) {
    //         #[cfg(feature = "physics")]
    //         if let Some(mut group) = self.groups.remove(index) {
    //             for (_, component_type) in group.types() {
    //                 for (_, c) in component_type.iter_mut() {
    //                     c.base_mut().deinit();
    //                 }
    //             }
    //         }

    //         self.force_update_sets = true;
    //         self.active_groups.remove(&index);
    //         return self.groups.remove(index);
    //     }
    //     return None;
    // }

    // pub fn active_render<'a, C: ComponentDerive>(
    //     &'a self,
    //     active: &ActiveComponents<C>,
    //     defaults: &'a GpuDefaults,
    // ) -> ComponentRenderGroup<'a, C> {
    //     let mut iters = vec![];
    //     let mut len = 0;

    //     for path in active.paths() {
    //         if let Some(group) = self.group(path.group_index) {
    //             if let Some(component_type) = group.type_ref(path.type_index) {
    //                 let type_len = component_type.len();
    //                 if type_len > 0 {
    //                     len += type_len;
    //                     iters.push((
    //                         component_type.buffer().unwrap_or(&defaults.empty_instance),
    //                         ComponentIterRender::new(component_type.iter().enumerate()),
    //                     ));
    //                 }
    //             }
    //         }
    //     }

    //     return ComponentRenderGroup::new(iters, len);
    // }

    // pub fn active<'a, C: ComponentDerive>(
    //     &'a self,
    //     active: &ActiveComponents<C>,
    // ) -> ComponentSet<'a, C> {
    //     let mut iters = vec![];
    //     let mut len = 0;

    //     for path in active.paths() {
    //         if let Some(group) = self.group(path.group_index) {
    //             if let Some(component_type) = group.type_ref(path.type_index) {
    //                 let type_len = component_type.len();
    //                 if type_len > 0 {
    //                     len += type_len;
    //                     iters.push(component_type.iter());
    //                 }
    //             }
    //         }
    //     }

    //     return ComponentSet::new(iters, len);
    // }

    // pub fn active_mut<'a, C: ComponentDerive>(
    //     &'a mut self,
    //     active: &ActiveComponents<C>,
    // ) -> ComponentSetMut<'a, C> {
    //     let mut iters = vec![];
    //     let mut len = 0;

    //     let mut head: &mut [ArenaEntry<_>] = self.groups.as_slice();
    //     let mut offset = 0;
    //     for path in active.paths() {
    //         let split = head.split_at_mut(path.group_index.index() as usize + 1 - offset);
    //         head = split.1;
    //         offset += split.0.len();
    //         match split.0.last_mut().unwrap() {
    //             ArenaEntry::Occupied { data, .. } => {
    //                 if let Some(component_type) = data.type_mut(path.type_index) {
    //                     let type_len = component_type.len();
    //                     if type_len > 0 {
    //                         len += type_len;
    //                         iters.push(component_type.iter_mut());
    //                     }
    //                 }
    //             }
    //             _ => unreachable!(),
    //         };
    //     }

    //     return ComponentSetMut::new(iters, len);
    // }

    // pub fn components<'a, C: ComponentController>(
    //     &'a self,
    //     filter: ComponentFilter,
    // ) -> ComponentSet<'a, C> {
    //     let type_id = C::IDENTIFIER;
    //     let mut iters = vec![];
    //     let mut len = 0;

    //     match filter {
    //         ComponentFilter::All => {
    //             for (_, group) in &self.groups {
    //                 if let Some(type_index) = group.type_index(type_id) {
    //                     let component_type = group.type_ref(*type_index).unwrap();
    //                     let type_len = component_type.len();
    //                     if type_len > 0 {
    //                         len += type_len;
    //                         iters.push(component_type.iter());
    //                     }
    //                 }
    //             }
    //         }
    //         ComponentFilter::Active => {
    //             let key = (C::CONFIG.priority, C::IDENTIFIER);
    //             if let Some(cluster) = self.active_components.get(&key) {
    //                 return self.active(&ActiveComponents::new(cluster.paths()));
    //             }
    //         }
    //         ComponentFilter::Specific(group_ids) => {
    //             for group_id in group_ids {
    //                 if let Some(index) = self.group_map.get(&group_id) {
    //                     let group = self.groups.get(*index).unwrap();

    //                     if let Some(type_index) = group.type_index(type_id) {
    //                         let component_type = group.type_ref(*type_index).unwrap();
    //                         let type_len = component_type.len();
    //                         if type_len > 0 {
    //                             len += type_len;
    //                             iters.push(component_type.iter());
    //                         }
    //                     };
    //                 }
    //             }
    //         }
    //     };

    //     return ComponentSet::new(iters, len);
    // }

    // pub fn components_mut<C: ComponentController>(
    //     &mut self,
    //     filter: ComponentFilter,
    // ) -> ComponentSetMut<C> {
    //     let type_id = C::IDENTIFIER;
    //     let mut iters = vec![];
    //     let mut len = 0;

    //     match filter {
    //         ComponentFilter::All => {
    //             for (_, group) in &mut self.groups {
    //                 if let Some(type_index) = group.type_index(type_id) {
    //                     let component_type = group.type_mut(*type_index).unwrap();
    //                     let type_len = component_type.len();
    //                     if type_len > 0 {
    //                         len += type_len;
    //                         iters.push(component_type.iter_mut());
    //                     }
    //                 }
    //             }
    //         }
    //         ComponentFilter::Active => {
    //             let key = (C::CONFIG.priority, C::IDENTIFIER);
    //             if let Some(cluster) = self.active_components.get(&key).cloned() {
    //                 return self.active_mut(&ActiveComponents::new(&cluster.paths()));
    //             }
    //         }
    //         ComponentFilter::Specific(group_ids) => {
    //             let mut indices: Vec<ArenaIndex> = group_ids
    //                 .iter()
    //                 .filter_map(|group_id| self.group_index(group_id).cloned())
    //                 .collect();
    //             indices.sort_by(|a, b| a.index().cmp(&b.index()));
    //             let mut head: &mut [ArenaEntry<_>] = self.groups.as_slice();
    //             let mut offset = 0;
    //             for index in &indices {
    //                 let split = head.split_at_mut(index.index() as usize + 1 - offset);
    //                 head = split.1;
    //                 offset += split.0.len();
    //                 match split.0.last_mut().unwrap() {
    //                     ArenaEntry::Occupied { data, .. } => {
    //                         if let Some(type_index) = data.type_index(type_id) {
    //                             let component_type = data.type_mut(*type_index).unwrap();
    //                             let type_len = component_type.len();
    //                             if type_len > 0 {
    //                                 len += type_len;
    //                                 iters.push(component_type.iter_mut());
    //                             }
    //                         }
    //                     }
    //                     _ => unreachable!(),
    //                 };
    //             }
    //         }
    //     };

    //     return ComponentSetMut::new(iters, len);
    // }

    // pub fn boxed_component(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
    //     if let Some(group) = self.groups.get(handle.group_index()) {
    //         if let Some(component_type) = group.type_ref(handle.type_index()) {
    //             return component_type.component(handle.component_index());
    //         }
    //     }
    //     return None;
    // }

    // pub fn boxed_component_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
    //     if let Some(group) = self.groups.get_mut(handle.group_index()) {
    //         if let Some(component_type) = group.type_mut(handle.type_index()) {
    //             return component_type.component_mut(handle.component_index());
    //         }
    //     }
    //     return None;
    // }

    // pub fn amount_of_components<C: ComponentController + ComponentDerive>(
    //     &self,
    //     group_id: ComponentGroupId,
    // ) -> usize {
    //     if let Some(group) = self.group_by_id(group_id) {
    //         if let Some(component_type) = group.type_by_id(C::IDENTIFIER) {
    //             return component_type.len();
    //         }
    //     }
    //     return 0;
    // }

    // pub fn component_by_index<C: ComponentController + ComponentDerive>(
    //     &self,
    //     group_id: ComponentGroupId,
    //     index: u32,
    // ) -> Option<&C> {
    //     if let Some(group) = self.group_by_id(group_id) {
    //         if let Some(component_type) = group.type_by_id(C::IDENTIFIER) {
    //             if let Some(component) = component_type.index(index as usize) {
    //                 return component.as_ref().downcast_ref();
    //             }
    //         }
    //     }
    //     return None;
    // }

    // pub fn component_by_index_mut<C: ComponentController + ComponentDerive>(
    //     &mut self,
    //     group_id: ComponentGroupId,
    //     index: u32,
    // ) -> Option<&mut C> {
    //     if let Some(group) = self.group_by_id_mut(group_id) {
    //         if let Some(component_type) = group.type_by_id_mut(C::IDENTIFIER) {
    //             if let Some(component) = component_type.index_mut(index as usize) {
    //                 return component.as_mut().downcast_mut();
    //             }
    //         }
    //     }
    //     return None;
    // }

    // pub fn component<C: ComponentDerive>(&self, handle: ComponentHandle) -> Option<&C> {
    //     if let Some(group) = self.groups.get(handle.group_index()) {
    //         if let Some(component_type) = group.type_ref(handle.type_index()) {
    //             if let Some(component) = component_type.component(handle.component_index()) {
    //                 return component.as_ref().downcast_ref();
    //             }
    //         }
    //     }
    //     return None;
    // }

    // pub fn component_mut<C: ComponentDerive>(&mut self, handle: ComponentHandle) -> Option<&mut C> {
    //     if let Some(group) = self.groups.get_mut(handle.group_index()) {
    //         if let Some(component_type) = group.type_mut(handle.type_index()) {
    //             if let Some(component) = component_type.component_mut(handle.component_index()) {
    //                 return component.as_mut().downcast_mut();
    //             }
    //         }
    //     }
    //     return None;
    // }

    // pub fn does_group_exist(&self, group: ComponentGroupId) -> bool {
    //     self.group_map.contains_key(&group)
    // }

    // pub(crate) fn group_mut(&mut self, index: ArenaIndex) -> Option<&mut ComponentGroup> {
    //     self.groups.get_mut(index)
    // }

    // pub(crate) fn group(&self, index: ArenaIndex) -> Option<&ComponentGroup> {
    //     self.groups.get(index)
    // }

    // pub(crate) fn group_index(&self, id: &ComponentGroupId) -> Option<&ArenaIndex> {
    //     return self.group_map.get(id);
    // }

    // pub fn active_group_ids(&self) -> &[ComponentGroupId] {
    //     return &self.active_group_ids;
    // }

    // pub fn group_ids(&self) -> impl Iterator<Item = &ComponentGroupId> {
    //     self.group_map.keys()
    // }

    // pub fn groups(&self) -> impl Iterator<Item = &ComponentGroup> {
    //     self.groups.iter().map(|(_, group)| group)
    // }

    // pub fn groups_mut(&mut self) -> impl Iterator<Item = &mut ComponentGroup> {
    //     self.groups.iter_mut().map(|(_, group)| group)
    // }

    // pub fn group_by_id(&self, id: ComponentGroupId) -> Option<&ComponentGroup> {
    //     if let Some(group_index) = self.group_index(&id) {
    //         return self.group(*group_index);
    //     }
    //     return None;
    // }

    // pub fn group_by_id_mut(&mut self, id: ComponentGroupId) -> Option<&mut ComponentGroup> {
    //     if let Some(group_index) = self.group_index(&id) {
    //         return self.group_mut(*group_index);
    //     }
    //     return None;
    // }

    // #[cfg(feature = "physics")]
    // pub(crate) fn component_callbacks(&self, type_id: &ComponentTypeId) -> &ComponentCallbacks {
    //     return self.component_callbacks.get(&type_id).unwrap();
    // }

    // pub(crate) fn copy_active_components(
    //     &self,
    // ) -> Rc<BTreeMap<(i16, ComponentTypeId), ComponentCluster>> {
    //     return self.active_components.clone();
    // }

    // pub fn group_deltas(&self) -> &[GroupDelta] {
    //     &self.group_deltas
    // }

    // pub fn instance_buffer<C: ComponentController>(
    //     &self,
    //     group_id: ComponentGroupId,
    // ) -> Option<&InstanceBuffer> {
    //     if let Some(group_index) = self.group_map.get(&group_id) {
    //         let group = self.groups.get(*group_index).unwrap();
    //         if let Some(component_type_index) = group.type_index(C::IDENTIFIER) {
    //             let component_type = group.type_ref(*component_type_index).unwrap();
    //             return component_type.buffer();
    //         }
    //     }
    //     return None;
    // }

    // #[cfg(feature = "physics")]
    // pub(crate) fn collision_event(
    //     &mut self,
    // ) -> Result<rapier2d::prelude::CollisionEvent, crossbeam::channel::TryRecvError> {
    //     self.world.borrow_mut().collision_event()
    // }

    // #[cfg(feature = "serde")]
    // pub(crate) fn register_callbacks<C: ComponentController>(&mut self) {
    //     self.component_callbacks
    //         .insert(C::IDENTIFIER, ComponentCallbacks::new::<C>());
    // }

    // #[cfg(feature = "serde")]
    // pub(crate) fn serialize_groups(
    //     &self,
    //     filter: ComponentFilter,
    // ) -> Vec<Option<(&u32, &ComponentGroup)>> {
    //     let mut ids = FxHashSet::default();
    //     match filter {
    //         ComponentFilter::All => {
    //             for group_id in self.group_ids() {
    //                 ids.insert(*group_id);
    //             }
    //         }
    //         ComponentFilter::Active => {
    //             for group_id in self.active_group_ids() {
    //                 ids.insert(*group_id);
    //             }
    //         }
    //         ComponentFilter::Specific(group_ids) => {
    //             for group_id in group_ids {
    //                 ids.insert(*group_id);
    //             }
    //         }
    //     }
    //     return self.groups.serialize_groups(ids);
    // }

    // #[cfg(feature = "serde")]
    // pub(crate) fn deserialize_groups(&mut self, groups: Arena<ComponentGroup>) {
    //     for (index, group) in &groups {
    //         self.group_map.insert(group.id(), index);
    //     }
    //     self.groups = groups;
    // }
}
