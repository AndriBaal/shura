use rustc_hash::FxHashMap;

#[cfg(feature = "log")]
use crate::log::info;
#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    Arena, BoxedComponent, BufferOperation, CallableType, ComponentConfig, ComponentController,
    ComponentHandle, ComponentSet, ComponentSetMut, ComponentType, ComponentTypeId, Gpu,
    GroupHandle, GroupManager, InstanceBuffer, InstanceIndex, InstanceIndices, RenderCamera,
    RenderOperation, Renderer, TypeIndex, UpdateOperation,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[macro_export]
/// Register multiple components at once
macro_rules! register {
    ($components: expr, $groups: expr, [$($C:ty),*]) => {
        {
            $(
                $components.register::<$C>($groups);
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
                $components.set::<$C>(),
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

pub struct RawSet<'a> {
    component_type: &'a mut ComponentType,
    groups: &'a [GroupHandle],
    check: bool,
}

impl<'a> RawSet<'a> {
    pub fn cast<C: ComponentController>(self) -> ComponentSetMut<'a, C> {
        ComponentSetMut::new(self.component_type, self.groups, self.check)
    }
}

#[macro_export]
/// Wrapper around unsafeties of getting multiple [ComponentSets](crate::ComponentSetMut)
macro_rules! sets_mut {
    ($components: expr, [$($C:ty),*]) => {
        {
            let raw: Vec<_> = $components.sets_mut(&[$((shura::ComponentFilter::Active, <$C>::IDENTIFIER), )*]);
            let mut iter = raw.into_iter();
            (
                $(
                    iter.next().unwrap().cast::<$C>(),
                )*
            )
        }
    };
}

#[macro_export]
/// Wrapper around unsafeties of getting multiple [ComponentSets](crate::ComponentSetMut)
macro_rules! sets_mut_of {
    ($components: expr, [$(($filter: expr, $C:ty)),*]) => {
        {
            let raw: Vec<_> = $components.sets_mut(&[$(($filter, <$C>::IDENTIFIER), )*]);
            let mut iter = raw.into_iter();
            (
                $(
                    iter.next().unwrap().cast::<$C>(),
                )*
            )
        }
    };
}

macro_rules! group_filter {
    ($self:ident, $filter: expr) => {
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
    pub(super) types: Arena<ComponentType>,

    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    callables: FxHashMap<TypeIndex, CallableType>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    update_priorities: Rc<RefCell<BTreeMap<(i16, ComponentTypeId), TypeIndex>>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    render_priorities: Rc<RefCell<BTreeMap<(i16, ComponentTypeId), TypeIndex>>>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    new_priorities: Vec<(Option<i16>, Option<i16>, ComponentTypeId, TypeIndex)>,
    pub(super) active_groups: Vec<GroupHandle>,
    pub(super) all_groups: Vec<GroupHandle>,
}

impl ComponentManager {
    pub(crate) fn new() -> Self {
        Self {
            types: Default::default(),
            type_map: Default::default(),
            callables: Default::default(),
            new_priorities: Default::default(),
            update_priorities: Default::default(),
            render_priorities: Default::default(),
            all_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            active_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
        }
    }

    pub(crate) fn buffer(&mut self, #[cfg(feature = "physics")] world: &World, gpu: &Gpu) {
        for (idx, ty) in self.types.iter_with_index_mut() {
            let callable = self.callables.get(&TypeIndex(idx)).unwrap();

            if ty.config().buffer != BufferOperation::Never {
                ty.buffer(
                    #[cfg(feature = "physics")]
                    world,
                    callable.callbacks.buffer,
                    &self.active_groups,
                    &gpu,
                );
            }
        }
    }

    pub(crate) fn apply_priorities(&mut self) {
        if self.new_priorities.len() > 0 {
            let mut update_priorities = self.update_priorities.borrow_mut();
            let mut render_priorities = self.render_priorities.borrow_mut();
            for (update_priority, render_priority, type_id, index) in self.new_priorities.drain(..)
            {
                if let Some(update_priority) = update_priority {
                    update_priorities.insert((update_priority, type_id), index);
                }
                if let Some(render_priority) = render_priority {
                    render_priorities.insert((render_priority, type_id), index);
                }
            }
        }
    }

    pub(crate) fn update_priorities(
        &self,
    ) -> Rc<RefCell<BTreeMap<(i16, ComponentTypeId), TypeIndex>>> {
        return self.update_priorities.clone();
    }

    pub(crate) fn render_priorities(
        &self,
    ) -> Rc<RefCell<BTreeMap<(i16, ComponentTypeId), TypeIndex>>> {
        return self.render_priorities.clone();
    }

    pub(crate) fn callable(&self, t: &TypeIndex) -> &CallableType {
        self.callables.get(t).unwrap()
    }

    pub(crate) fn callable_mut(&mut self, t: &TypeIndex) -> &mut CallableType {
        self.callables.get_mut(t).unwrap()
    }

    #[cfg(feature = "serde")]
    pub(crate) fn reregister<C: ComponentController>(&mut self) {
        let index = *self.type_map.get(&C::IDENTIFIER).unwrap();
        #[cfg(feature = "log")]
        info!(
            "Reregister component '{}' with ID '{}'",
            C::TYPE_NAME,
            C::IDENTIFIER
        );
        self.new_priorities.push((
            if C::CONFIG.update != UpdateOperation::Never {
                Some(C::CONFIG.update_priority)
            } else {
                None
            },
            if C::CONFIG.render != RenderOperation::Never {
                Some(C::CONFIG.render_priority)
            } else {
                None
            },
            C::IDENTIFIER,
            index,
        ));
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

    pub fn register<C: ComponentController>(&mut self, groups: &GroupManager) {
        self.register_with_config::<C>(groups, C::CONFIG);
    }

    pub fn type_id(&self, component: ComponentHandle) -> ComponentTypeId {
        self.callables.get(&component.type_index()).unwrap().type_id
    }

    pub fn register_with_config<C: ComponentController>(
        &mut self,
        groups: &GroupManager,
        config: ComponentConfig,
    ) {
        if !self.type_map.contains_key(&C::IDENTIFIER) {
            let index = self
                .types
                .insert_with(|idx| ComponentType::with_config::<C>(config, TypeIndex(idx), groups));
            #[cfg(feature = "log")]
            info!(
                "Register component '{}' with ID '{}'",
                C::TYPE_NAME,
                C::IDENTIFIER
            );
            self.type_map.insert(C::IDENTIFIER, TypeIndex(index));
            self.new_priorities.push((
                if C::CONFIG.update != UpdateOperation::Never {
                    Some(C::CONFIG.update_priority)
                } else {
                    None
                },
                if C::CONFIG.render != RenderOperation::Never {
                    Some(C::CONFIG.render_priority)
                } else {
                    None
                },
                C::IDENTIFIER,
                TypeIndex(index),
            ));
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

    pub fn is_type_of<C: ComponentController>(&self, component: ComponentHandle) -> bool {
        if let Some(ty) = self.type_map.get(&C::IDENTIFIER) {
            return component.type_index() == *ty;
        }
        return false;
    }

    pub fn component_type_id(&self, component: ComponentHandle) -> ComponentTypeId {
        return self.types[component.type_index().0].component_type_id();
    }

    pub fn index<C: ComponentController>(&self, index: usize) -> Option<&C> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_mut<C: ComponentController>(&mut self, index: usize) -> Option<&mut C> {
        self.index_mut_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_of<C: ComponentController>(&self, group: GroupHandle, index: usize) -> Option<&C> {
        let ty = type_ref!(self, C);
        ty.index(group, index)
    }

    pub fn index_mut_of<C: ComponentController>(
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

    pub fn remove<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
    ) -> Option<C> {
        self.types.get_mut(handle.type_index().0).unwrap().remove(
            #[cfg(feature = "physics")]
            world,
            handle,
        )
    }

    pub fn remove_boxed(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
    ) -> Option<BoxedComponent> {
        self.types
            .get_mut(handle.type_index().0)
            .unwrap()
            .remove_boxed(
                #[cfg(feature = "physics")]
                world,
                handle,
            )
    }

    #[inline]
    pub fn add<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        component: C,
    ) -> ComponentHandle {
        self.add_to(
            #[cfg(feature = "physics")]
            world,
            GroupHandle::DEFAULT_GROUP,
            component,
        )
    }

    pub fn add_to<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.add(
            #[cfg(feature = "physics")]
            world,
            group_handle,
            component,
        )
    }

    #[inline]
    pub fn add_many<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.add_many_to(
            #[cfg(feature = "physics")]
            world,
            GroupHandle::DEFAULT_GROUP,
            components,
        )
    }

    pub fn add_many_to<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let ty = type_mut!(self, C);
        ty.add_many::<C>(
            #[cfg(feature = "physics")]
            world,
            group_handle,
            components,
        )
    }

    #[inline]
    pub fn add_with<C: ComponentController + ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.add_with_to(
            #[cfg(feature = "physics")]
            world,
            GroupHandle::DEFAULT_GROUP,
            create,
        )
    }

    pub fn add_with_to<C: ComponentController + ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.add_with::<C>(
            #[cfg(feature = "physics")]
            world,
            group_handle,
            create,
        )
    }

    #[inline]
    pub fn remove_all<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
    ) -> Vec<C> {
        self.remove_all_of(
            #[cfg(feature = "physics")]
            world,
            ComponentFilter::All,
        )
    }

    pub fn remove_all_of<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        filter: ComponentFilter,
    ) -> Vec<C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.remove_all(
            #[cfg(feature = "physics")]
            world,
            groups,
        )
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

    #[inline]
    pub fn iter_render<C: ComponentController>(
        &self,
    ) -> impl DoubleEndedIterator<Item = (&InstanceBuffer, InstanceIndex, &C)> {
        self.iter_render_of::<C>(ComponentFilter::Active)
    }

    pub fn iter_render_of<C: ComponentController>(
        &self,
        filter: ComponentFilter,
    ) -> impl DoubleEndedIterator<Item = (&InstanceBuffer, InstanceIndex, &C)> {
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

    pub fn sets_mut<'a>(
        &'a mut self,
        ids: &[(ComponentFilter<'a>, ComponentTypeId)],
    ) -> Vec<RawSet<'a>> {
        let mut result = Vec::with_capacity(ids.len());
        for (index, value) in ids.iter().enumerate() {
            for other in ids.iter().skip(index + 1) {
                if value == other {
                    panic!("Duplicate component type!");
                }
            }
        }

        // We can use unsafe here because we validate for no duplicates
        unsafe {
            for (filter, id) in ids {
                let filter = *filter;
                let (check, groups) = group_filter!(self, filter);
                let idx = self.type_map.get(id).unwrap();
                let ty = self.types.get(idx.0).unwrap();
                let ptr = ty as *const _ as *mut _;
                let component_type = &mut *ptr;
                result.push(RawSet {
                    groups,
                    component_type,
                    check,
                });
            }
        }
        return result;
    }

    pub fn set_mut_of<'a, C: ComponentController>(
        &'a mut self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    #[inline]
    pub fn for_each<C: ComponentController>(&self, each: impl FnMut(&C)) {
        self.for_each_of(ComponentFilter::Active, each)
    }

    pub fn for_each_of<C: ComponentController>(
        &self,
        filter: ComponentFilter,
        each: impl FnMut(&C),
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.for_each(groups, each);
    }

    #[inline]
    pub fn for_each_mut<C: ComponentController>(&mut self, each: impl FnMut(&mut C)) {
        self.for_each_mut_of(ComponentFilter::Active, each)
    }


    #[inline]
    pub fn par_for_each<C: ComponentController>(
        &self,
        each: impl Fn(&C) + Send + Sync,
    ) {
        self.par_for_each_of(ComponentFilter::Active, each)
    }

    pub fn par_for_each_of<C: ComponentController>(
        &self,
        filter: ComponentFilter,
        each: impl Fn(&C) + Send + Sync,
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.par_for_each(groups, each);
    }
    
    pub fn for_each_mut_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
        each: impl FnMut(&mut C),
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.for_each_mut(groups, each);
    }

    #[inline]
    pub fn par_for_each_mut<C: ComponentController>(
        &mut self,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        self.par_for_each_mut_of(ComponentFilter::Active, each)
    }

    pub fn par_for_each_mut_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.par_for_each_mut(groups, each);
    }


    #[inline]
    pub fn retain<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        #[cfg(feature = "physics")] keep: impl FnMut(&mut C, &mut World) -> bool,
        #[cfg(not(feature = "physics"))] keep: impl FnMut(&mut C) -> bool,
    ) {
        self.retain_of(
            #[cfg(feature = "physics")]
            world,
            ComponentFilter::Active,
            keep,
        )
    }

    pub fn retain_of<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        filter: ComponentFilter,
        #[cfg(feature = "physics")] keep: impl FnMut(&mut C, &mut World) -> bool,
        #[cfg(not(feature = "physics"))] keep: impl FnMut(&mut C) -> bool,
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_mut!(self, C);
        ty.retain(
            #[cfg(feature = "physics")]
            world,
            groups,
            keep,
        );
    }

    pub fn render_each<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        each: impl FnMut(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        let ty = type_ref!(self, C);
        ty.render_each(renderer, camera, each)
    }

    pub fn render_single<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        each: impl FnOnce(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        let ty = type_ref!(self, C);
        ty.render_single(renderer, camera, each)
    }

    pub fn render_each_prepare<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        prepare: impl FnOnce(&mut Renderer<'a>),
        each: impl FnMut(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        let ty = type_ref!(self, C);
        ty.render_each_prepare(renderer, camera, prepare, each)
    }

    pub fn render_all<'a, C: ComponentController>(
        &'a self,
        renderer: &mut Renderer<'a>,
        camera: RenderCamera<'a>,
        all: impl FnMut(&mut Renderer<'a>, InstanceIndices),
    ) {
        let ty = type_ref!(self, C);
        ty.render_all::<C>(renderer, camera, all)
    }

    pub fn single<C: ComponentController>(&self) -> Option<&C> {
        let ty = type_ref!(self, C);
        ty.single()
    }

    pub fn single_mut<C: ComponentController>(&mut self) -> Option<&mut C> {
        let ty = type_mut!(self, C);
        ty.single_mut()
    }

    pub fn remove_single<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
    ) -> Option<C> {
        let ty = type_mut!(self, C);
        ty.remove_single(
            #[cfg(feature = "physics")]
            world,
        )
    }

    pub fn set_single<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        new: C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.set_single(
            #[cfg(feature = "physics")]
            world,
            new,
        )
    }

    pub fn set_single_with<C: ComponentController>(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let ty = type_mut!(self, C);
        ty.set_single_with(
            #[cfg(feature = "physics")]
            world,
            create,
        )
    }
}
