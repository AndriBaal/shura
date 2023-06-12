use rustc_hash::FxHashMap;

#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Arena, BoxedComponent, CallableType, CameraBuffer, ComponentConfig, ComponentController,
    ComponentHandle, ComponentSet, ComponentSetMut, ComponentType, ComponentTypeId, Gpu, Group,
    GroupActivation, GroupHandle, InstanceBuffer, InstanceIndex, TypeIndex, Vector,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[macro_export]
/// Register multiple components at once
macro_rules! register {
    ($components: expr,[$($C:ty),*]) => {
        {
            $(
                $components.register::<$C>();
            )*
        }
    };
}

#[macro_export]
/// Wrapper for getting multiple [ComponentSets](crate::ComponentSet)
macro_rules! sets {
    ($components: expr, [$($C:ty),*]) => {
        {
            ($(
                $components.set::<$C>($filter),
            )*)
        }
    };
}

#[macro_export]
/// Wrapper for getting multiple [ComponentSets](crate::ComponentSet) from the specified groups
macro_rules! sets_of {
    ($components: expr, [$(($filter: expr, $C:ty)),*]) => {
        {
            ($(
                $components.set_of::<$C>($filter),
            )*)
        }
    }
}

#[macro_export]
/// Wrapper around unsafeties of getting multiple [ComponentSets](crate::ComponentSetMut)
macro_rules! sets_mut {
    ($components: expr, [$($C:ty),*]) => {
        {
            if !shura::ComponentManager::validate(&[
                $(
                    <$C>::IDENTIFIER,
                )*
            ]) {
                panic!("Duplicate component type!");
            }
            let ptr: *mut shura::ComponentManager = $components as *mut _;
            unsafe {
                ($(
                    (&mut *ptr).set_mut::<$C>(),
                )*)
            }
        }
    };
}

#[macro_export]
/// Wrapper around unsafeties of getting multiple [ComponentSets](crate::ComponentSetMut) from the specified groups
macro_rules! sets_mut_of {
    ($components: expr, [$(($filter: expr, $C:ty)),*]) => {
        {
            if !shura::ComponentManager::validate(&[
                $(
                    <$C>::IDENTIFIER,
                )*
            ]) {
                panic!("Duplicate component type!");
            }
            let ptr: *mut shura::ComponentManager = $components as *mut _;
            unsafe {
                ($(
                    (&mut *ptr).set_mut_of::<$C>($filter),
                )*)
            }
        }
    };
}

macro_rules! group_filter {
    ($self:ident, $filter: ident) => {
        match $filter {
            ComponentFilter::All => (false, &$self.all_groups[..]),
            ComponentFilter::Active => (false, &$self.active_groups[..]),
            ComponentFilter::Specific(h) => (true, h),
        }
    };
}

fn no_type_error<C: ComponentController>() -> String {
    format!("The type '{}' first needs to be registered!", C::TYPE_NAME)
}

macro_rules! type_ref {
    ($self:ident, $C: ident) => {{
        let idx = $self
            .type_map
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>());
        let ty = $self.types.get(idx.0).unwrap();
        ty
    }};
}

macro_rules! type_mut {
    ($self:ident, $C: ident) => {{
        let idx = $self
            .type_map
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>());
        let ty = $self.types.get_mut(idx.0).unwrap();
        ty
    }};
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
/// Filter components by groups
pub enum ComponentFilter<'a> {
    All,
    Active,
    Specific(&'a [GroupHandle]),
}

impl<'a> Default for ComponentFilter<'a> {
    fn default() -> Self {
        return ComponentFilter::Active;
    }
}

impl ComponentFilter<'static> {
    pub const DEFAULT_GROUP: Self = ComponentFilter::Specific(&[GroupHandle::DEFAULT_GROUP]);
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Access to the component system
pub struct ComponentManager {
    type_map: FxHashMap<ComponentTypeId, TypeIndex>,
    types: Arena<ComponentType>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    callables: FxHashMap<TypeIndex, CallableType>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    priorities: Rc<RefCell<BTreeMap<(i16, ComponentTypeId), TypeIndex>>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    new_priorities: Vec<(i16, ComponentTypeId, TypeIndex)>,
    groups: Arena<Group>,
    active_groups: Vec<GroupHandle>,
    all_groups: Vec<GroupHandle>,
}

impl ComponentManager {
    pub(crate) fn new() -> Self {
        let default_component_group = Group::new(GroupActivation::Always, 0);
        let mut groups = Arena::default();
        let index = groups.insert(default_component_group);
        let group_handle = GroupHandle(index);
        Self {
            types: Default::default(),
            type_map: Default::default(),
            callables: Default::default(),
            priorities: Default::default(),
            new_priorities: Default::default(),
            all_groups: Vec::from_iter([group_handle]),
            active_groups: Vec::from_iter([group_handle]),
            groups,
        }
    }

    pub(crate) fn update_sets(&mut self, camera: &CameraBuffer) {
        let cam_aabb = camera.model().aabb(Vector::new(0.0, 0.0).into()); // Translation is already applied
        self.active_groups.clear();
        for (index, group) in &mut self.groups {
            if group.intersects_camera(cam_aabb) {
                group.set_active(true);
                self.active_groups.push(GroupHandle(index));
            }
        }
    }

    pub(crate) fn buffer(&mut self, #[cfg(feature = "physics")] world: &mut World, gpu: &Gpu) {
        for (_, ty) in &mut self.types {
            ty.buffer(
                #[cfg(feature = "physics")]
                world,
                &self.active_groups,
                gpu,
            );
        }
    }

    pub(crate) fn priorities(
        &mut self,
    ) -> Rc<RefCell<BTreeMap<(i16, ComponentTypeId), TypeIndex>>> {
        if self.new_priorities.len() > 0 {
            let mut priorities = self.priorities.borrow_mut();
            for (priority, type_id, index) in self.new_priorities.drain(..) {
                priorities.insert((priority, type_id), index);
            }
        }
        return self.priorities.clone();
    }

    #[cfg(feature = "physics")]
    pub(crate) fn callable(&self, t: &TypeIndex) -> &CallableType {
        self.callables.get(t).unwrap()
    }

    pub(crate) fn callable_mut(&mut self, t: &TypeIndex) -> &mut CallableType {
        self.callables.get_mut(t).unwrap()
    }

    #[cfg(feature = "serde")]
    pub(crate) fn reregister<C: ComponentController>(&mut self) {
        let index = *self.type_map.get(&C::IDENTIFIER).unwrap();
        self.new_priorities
            .push((C::CONFIG.priority, C::IDENTIFIER, index));
        self.callables.insert(index, CallableType::new::<C>());
    }

    #[cfg(feature = "serde")]
    pub(crate) fn type_ref<C: ComponentController>(&self) -> &ComponentType {
        type_ref!(self, C)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn type_mut<C: ComponentController>(&mut self) -> &mut ComponentType {
        type_mut!(self, C)
    }

    #[cfg(all(feature = "serde", feature = "physics"))]
    pub(crate) fn types(&self) -> &Arena<ComponentType> {
        &self.types
    }

    #[cfg(feature = "physics")]
    pub fn apply_world_mapping(&mut self, world: &mut World) {
        for (_, ty) in &mut self.types {
            ty.apply_world_mapping(world)
        }
    }

    pub fn register<C: ComponentController>(&mut self) {
        self.register_with_config::<C>(C::CONFIG);
    }

    pub fn register_with_config<C: ComponentController>(&mut self, config: ComponentConfig) {
        if !self.type_map.contains_key(&C::IDENTIFIER) {
            let index = self.types.insert_with(|idx| {
                ComponentType::with_config::<C>(config, TypeIndex(idx), &self.groups)
            });
            #[cfg(feature = "log")]
            info!(
                "Register component '{}' with priority {} and ID '{}'",
                C::TYPE_NAME,
                C::CONFIG.priority,
                C::IDENTIFIER
            );
            self.type_map.insert(C::IDENTIFIER, TypeIndex(index));
            self.new_priorities
                .push((C::CONFIG.priority, C::IDENTIFIER, TypeIndex(index)));
            self.callables
                .insert(TypeIndex(index), CallableType::new::<C>());
        }
    }

    pub fn active_groups(&self) -> &[GroupHandle] {
        &self.active_groups
    }

    pub fn all_groups(&self) -> &[GroupHandle] {
        &self.all_groups
    }

    pub fn groups(&self) -> impl Iterator<Item = (GroupHandle, &Group)> + Clone {
        return self
            .groups
            .iter()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn groups_mut(&mut self) -> impl Iterator<Item = (GroupHandle, &mut Group)> {
        return self
            .groups
            .iter_mut()
            .map(|(index, group)| (GroupHandle(index), group));
    }

    pub fn contains_group(&self, handle: GroupHandle) -> bool {
        return self.groups.contains(handle.0);
    }

    pub fn group(&self, handle: GroupHandle) -> Option<&Group> {
        return self.groups.get(handle.0);
    }

    pub fn group_mut(&mut self, handle: GroupHandle) -> Option<&mut Group> {
        return self.groups.get_mut(handle.0);
    }

    pub fn is_type_of<C: ComponentController>(&self, component: ComponentHandle) -> bool {
        if let Some(ty) = self.type_map.get(&C::IDENTIFIER) {
            return component.type_index() == *ty;
        }
        return false;
    }

    pub fn type_id_of(&self, component: ComponentHandle) -> ComponentTypeId {
        return self.types[component.type_index().0].component_type_id();
    }

    pub fn add_group(&mut self, group: Group) -> GroupHandle {
        let handle = GroupHandle(self.groups.insert(group));
        for (_, ty) in &mut self.types {
            let result = ty.add_group();
            assert_eq!(handle, result);
        }
        self.all_groups.push(handle);
        return handle;
    }

    pub fn remove_group(&mut self, handle: GroupHandle) -> Option<Group> {
        if handle == GroupHandle::DEFAULT_GROUP {
            panic!("Cannot remove default group!");
        }
        let group = self.groups.remove(handle.0);
        for (_, ty) in &mut self.types {
            ty.remove_group(handle);
        }
        self.all_groups.retain(|h| *h != handle);
        return group;
    }

    pub fn index<C: ComponentController>(&self, group: GroupHandle, index: usize) -> Option<&C> {
        let ty = type_ref!(self, C);
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
        self.types
            .get_mut(handle.type_index().0)
            .unwrap()
            .get_mut(handle)
    }

    pub fn get2_mut<C1: ComponentController, C2: ComponentController>(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut C1>, Option<&mut C2>) {
        assert_ne!(handle1, handle2);
        if handle1.type_index() == handle2.type_index() {
            let ty = self.types.get_mut(handle1.type_index().0).unwrap();
            return ty.get2_mut::<C1, C2>(handle1, handle2);
        } else {
            let (ty1, ty2) = self
                .types
                .get2_mut(handle1.type_index().0, handle2.type_index().0);
            return (
                ty1.unwrap().get_mut::<C1>(handle1),
                ty2.unwrap().get_mut::<C2>(handle2),
            );
        }
    }

    pub fn get2_mut_boxed(
        &mut self,
        handle1: ComponentHandle,
        handle2: ComponentHandle,
    ) -> (Option<&mut BoxedComponent>, Option<&mut BoxedComponent>) {
        assert_ne!(handle1, handle2);
        if handle1.type_index() == handle2.type_index() {
            let ty = self.types.get_mut(handle1.type_index().0).unwrap();
            return ty.get2_mut_boxed(handle1, handle2);
        } else {
            let (ty1, ty2) = self
                .types
                .get2_mut(handle1.type_index().0, handle2.type_index().0);
            return (
                ty1.unwrap().get_boxed_mut(handle1),
                ty2.unwrap().get_boxed_mut(handle2),
            );
        }
    }

    pub fn get_boxed(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
        self.types
            .get(handle.type_index().0)
            .unwrap()
            .get_boxed(handle)
    }

    pub fn get_boxed_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
        self.types
            .get_mut(handle.type_index().0)
            .unwrap()
            .get_boxed_mut(handle)
    }

    pub fn remove<C: ComponentController>(&mut self, handle: ComponentHandle) -> Option<C> {
        self.types
            .get_mut(handle.type_index().0)
            .unwrap()
            .remove(handle)
    }

    pub fn remove_boxed(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
        self.types
            .get_mut(handle.type_index().0)
            .unwrap()
            .remove_boxed(handle)
    }

    #[inline]
    pub fn add<C: ComponentController>(&mut self, component: C) -> ComponentHandle {
        self.add_to(GroupHandle::DEFAULT_GROUP, component)
    }

    pub fn add_to<C: ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.add(group_handle, component)
    }

    #[inline]
    pub fn add_many<C: ComponentController>(
        &mut self,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.add_many_to(GroupHandle::DEFAULT_GROUP, components)
    }

    pub fn add_many_to<C: ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let ty = type_mut!(self, C);
        ty.add_many::<C>(group_handle, components)
    }

    #[inline]
    pub fn add_with<C: ComponentController + ComponentController>(
        &mut self,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.add_with_to(GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to<C: ComponentController + ComponentController>(
        &mut self,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.add_with::<C>(group_handle, create)
    }

    #[inline]
    pub fn remove_all<C: ComponentController>(&mut self) -> Vec<(GroupHandle, Vec<C>)> {
        self.remove_all_of(ComponentFilter::All)
    }

    pub fn remove_all_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
    ) -> Vec<(GroupHandle, Vec<C>)> {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.remove_all(groups)
    }

    #[inline]
    pub fn force_buffer<C: ComponentController>(&mut self) {
        self.force_buffer_of::<C>(ComponentFilter::All)
    }

    pub fn force_buffer_of<C: ComponentController>(&mut self, filter: ComponentFilter) {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.force_buffer(groups)
    }

    #[inline]
    pub fn len<C: ComponentController>(&self) -> usize {
        self.len_of::<C>(ComponentFilter::All)
    }

    pub fn len_of<C: ComponentController>(&self, filter: ComponentFilter) -> usize {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.len(groups)
    }

    #[inline]
    pub fn iter<C: ComponentController>(&self) -> impl DoubleEndedIterator<Item = &C> {
        self.iter_of::<C>(ComponentFilter::Active)
    }

    pub fn iter_of<C: ComponentController>(
        &self,
        filter: ComponentFilter,
    ) -> impl DoubleEndedIterator<Item = &C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.iter(groups)
    }

    #[inline]
    pub fn iter_mut<C: ComponentController>(&mut self) -> impl DoubleEndedIterator<Item = &mut C> {
        self.iter_mut_of::<C>(ComponentFilter::Active)
    }

    pub fn iter_mut_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
    ) -> impl DoubleEndedIterator<Item = &mut C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        ty.iter_mut(groups, check)
    }

    // pub fn iter_mut_and_groups<C: ComponentController>(
    //     &mut self,
    //     filter: ComponentFilter,
    // ) -> (
    //     impl DoubleEndedIterator<Item = &mut C>,
    //     impl Iterator<Item = (GroupHandle, &Group)> + Clone,
    // ) {
    //     let (check, groups) = group_filter!(self, filter);
    //     let ty = type_mut!(self, C);
    //     (
    //         ty.iter_mut(groups, check),
    //         self.groups
    //             .iter()
    //             .map(|(index, group)| (GroupHandle(index), group)),
    //     )
    // }

    #[inline]
    pub fn iter_render<C: ComponentController>(
        &self,
    ) -> impl DoubleEndedIterator<
        Item = (
            &InstanceBuffer,
            impl DoubleEndedIterator<Item = (InstanceIndex, &C)> + Clone,
        ),
    > {
        self.iter_render_of::<C>(ComponentFilter::Active)
    }

    pub fn iter_render_of<C: ComponentController>(
        &self,
        filter: ComponentFilter,
    ) -> impl DoubleEndedIterator<
        Item = (
            &InstanceBuffer,
            impl DoubleEndedIterator<Item = (InstanceIndex, &C)> + Clone,
        ),
    > {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.iter_render(groups)
    }

    #[inline]
    pub fn iter_with_handles<'a, C: ComponentController>(
        &'a self,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
        self.iter_with_handles_of::<C>(ComponentFilter::Active)
    }

    pub fn iter_with_handles_of<'a, C: ComponentController>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.iter_with_handles(groups)
    }

    #[inline]
    pub fn iter_mut_with_handles<'a, C: ComponentController>(
        &'a mut self,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
        self.iter_mut_with_handles_of::<C>(ComponentFilter::Active)
    }

    pub fn iter_mut_with_handles_of<'a, C: ComponentController>(
        &'a mut self,
        filter: ComponentFilter<'a>,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        ty.iter_mut_with_handles(groups, check)
    }

    #[inline]
    pub fn set<'a, C: ComponentController>(&'a self) -> ComponentSet<'a, C> {
        self.set_of(ComponentFilter::Active)
    }

    pub fn set_of<'a, C: ComponentController>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSet<'a, C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        return ComponentSet::new(ty, groups);
    }

    #[inline]
    pub fn set_mut<'a, C: ComponentController>(&'a mut self) -> ComponentSetMut<'a, C> {
        self.set_mut_of(ComponentFilter::Active)
    }

    pub fn set_mut_of<'a, C: ComponentController>(
        &'a mut self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    pub fn validate(identifiers: &[ComponentTypeId]) -> bool {
        for (index, value) in identifiers.iter().enumerate() {
            for other in identifiers.iter().skip(index + 1) {
                if value == other {
                    return false;
                }
            }
        }
        return true;
    }

    #[inline]
    pub fn each<C: ComponentController>(&self, each: impl FnMut(&C)) {
        self.each_of(ComponentFilter::Active, each)
    }

    pub fn each_of<C: ComponentController>(&self, filter: ComponentFilter, each: impl FnMut(&C)) {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.each(groups, each);
    }

    #[inline]
    pub fn each_mut<C: ComponentController>(&mut self, each: impl FnMut(&mut C)) {
        self.each_mut_of(ComponentFilter::Active, each)
    }

    pub fn each_mut_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
        each: impl FnMut(&mut C),
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.each_mut(groups, each);
    }

    #[inline]
    pub fn retain<C: ComponentController>(&mut self, keep: impl FnMut(&mut C) -> bool) {
        self.retain_of(ComponentFilter::Active, keep)
    }

    pub fn retain_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
        keep: impl FnMut(&mut C) -> bool,
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.retain(groups, keep);
    }
}
