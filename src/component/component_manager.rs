use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

use crate::{
    Component, ComponentConfig, ComponentHandle, ComponentScope, ComponentSet, ComponentSetMut,
    ComponentType, ComponentTypeId, ContextUse, ControllerManager, GlobalComponents, Gpu,
    GroupHandle, GroupManager, InstancePosition, World,
};

#[cfg(feature = "serde")]
use crate::{ComponentTypeGroup, ComponentTypeStorage};

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub(crate) trait ComponentTypeImplementation: Downcast {
    fn add_group(&mut self);
    fn remove_group(&mut self, world: &mut World, handle: GroupHandle);
    fn camera_target(&self, world: &World, handle: ComponentHandle) -> Option<InstancePosition>;
    fn buffer(&mut self, world: &World, active: &[GroupHandle], gpu: &Gpu);
    fn component_type_id(&self) -> ComponentTypeId;
    fn config(&self) -> ComponentConfig;
    #[cfg(feature = "serde")]
    fn deinit_non_serialized(&self, world: &mut World);
    #[cfg(feature = "serde")]
    fn remove_group_serialize(
        &mut self,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<Box<dyn std::any::Any>>;
}
impl_downcast!(ComponentTypeImplementation);

#[macro_export]
/// Register multiple components at once
macro_rules! register {
    ($ctx: expr, [$($C:ty),* $(,)?]) => {
        {
            $(
                $ctx.components.register::<$C>(&$ctx.groups);
            )*
        }
    };
}

macro_rules! group_filter {
    ($self:ident, $filter: expr) => {
        match $filter {
            GroupFilter::All => (false, &$self.all_groups[..]),
            GroupFilter::Active => (false, &$self.active_groups[..]),
            GroupFilter::Custom(h) => (true, h),
        }
    };
}

macro_rules! type_ref {
    ($self:ident, $C: ident) => {{
        let ty = $self
            .types
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            ._ref::<$C>();
        ty
    }};
}

macro_rules! type_ref_mut {
    ($self:ident, $C: ident) => {{
        assert!(
            $self.context_use == ContextUse::Update,
            "This operation is only allowed while updating!"
        );
        let ty = $self
            .types
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            .ref_mut::<$C>();
        ty
    }};
}

macro_rules! type_render {
    ($self:ident, $C: ident) => {{
        assert!(
            $self.context_use == ContextUse::Render,
            "This operation is only allowed while rendering!"
        );
        let ty = $self
            .types
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            .resource::<$C>();
        ty
    }};
}

const ALREADY_BORROWED: &'static str = "This type is already borrowed!";
fn no_type_error<C: Component>() -> String {
    format!("The type '{}' first needs to be registered!", C::TYPE_NAME)
}

pub(crate) enum ComponentTypeScope {
    Scene(Box<RefCell<dyn ComponentTypeImplementation>>),
    Global(Rc<RefCell<dyn ComponentTypeImplementation>>),
}

impl ComponentTypeScope {
    fn ref_mut_raw(&self) -> RefMut<dyn ComponentTypeImplementation> {
        match &self {
            ComponentTypeScope::Scene(scene) => scene.try_borrow_mut().expect(ALREADY_BORROWED),
            ComponentTypeScope::Global(global) => global.try_borrow_mut().expect(ALREADY_BORROWED),
        }
    }

    fn _ref<C: Component>(&self) -> Ref<ComponentType<C>> {
        match &self {
            ComponentTypeScope::Scene(scene) => {
                Ref::map(scene.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<ComponentType<C>>().unwrap()
                })
            }
            ComponentTypeScope::Global(global) => {
                Ref::map(global.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<ComponentType<C>>().unwrap()
                })
            }
        }
    }

    fn ref_mut<C: Component>(&self) -> RefMut<ComponentType<C>> {
        match &self {
            ComponentTypeScope::Scene(scene) => {
                RefMut::map(scene.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<ComponentType<C>>().unwrap()
                })
            }
            ComponentTypeScope::Global(global) => {
                RefMut::map(global.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<ComponentType<C>>().unwrap()
                })
            }
        }
    }

    fn resource<C: Component>(&self) -> &ComponentType<C> {
        // This is safe, because we disallow .borrow_mut() with the ContextUse
        unsafe {
            match &self {
                ComponentTypeScope::Scene(scene) => scene
                    .try_borrow_unguarded()
                    .unwrap()
                    .downcast_ref::<ComponentType<C>>()
                    .unwrap(),
                ComponentTypeScope::Global(global) => global
                    .try_borrow_unguarded()
                    .unwrap()
                    .downcast_ref::<ComponentType<C>>()
                    .unwrap(),
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
/// Filter components by groups
pub enum GroupFilter<'a> {
    All,
    Active,
    Custom(&'a [GroupHandle]),
}

impl<'a> Default for GroupFilter<'a> {
    fn default() -> Self {
        return GroupFilter::Active;
    }
}

impl GroupFilter<'static> {
    pub const DEFAULT_GROUP: Self = GroupFilter::Custom(&[GroupHandle::DEFAULT_GROUP]);
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Access to the component system
pub struct ComponentManager {
    context_use: ContextUse,
    pub(super) active_groups: Vec<GroupHandle>,
    pub(super) all_groups: Vec<GroupHandle>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) controllers: Rc<ControllerManager>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) types: FxHashMap<ComponentTypeId, ComponentTypeScope>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    global: GlobalComponents,
}

impl ComponentManager {
    pub(crate) fn new(global: GlobalComponents) -> Self {
        Self {
            types: Default::default(),
            global,
            controllers: Rc::new(ControllerManager::new()),
            context_use: ContextUse::Update,
            all_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            active_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
        }
    }

    pub(crate) fn with_use(&mut self, context_use: ContextUse) -> &mut Self {
        self.context_use = context_use;
        self
    }

    pub(crate) fn buffer(&mut self, world: &World, gpu: &Gpu) {
        #[cfg(feature = "rayon")]
        {
            use rayon::prelude::*;
            // This is safe here because we dont't expand the map and we don't access the same map entry twice
            unsafe impl Send for ComponentTypeScope {}
            unsafe impl Sync for ComponentTypeScope {}
            self.controllers.buffers().par_iter().for_each(|ty| {
                let ty = &self.types[ty];
                ty.ref_mut_raw().buffer(world, &self.active_groups, &gpu);
            });
        }

        #[cfg(not(feature = "rayon"))]
        for ty in self.controllers.buffers() {
            let ty = &self.types[ty];
            ty.ref_mut_raw().buffer(world, &self.active_groups, &gpu);
        }
    }

    pub fn active_groups(&self) -> &[GroupHandle] {
        &self.active_groups
    }

    pub fn all_groups(&self) -> &[GroupHandle] {
        &self.all_groups
    }

    pub(crate) fn types_mut(
        &mut self,
    ) -> impl Iterator<Item = RefMut<'_, dyn ComponentTypeImplementation>> {
        self.types.values_mut().map(|r| r.ref_mut_raw())
    }

    pub fn register<C: Component>(&mut self, groups: &GroupManager) {
        self.register_with_config::<C>(groups, C::CONFIG);
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize_group<C: Component + serde::de::DeserializeOwned>(
        &mut self,
        mut storage: ComponentTypeGroup<C>,
        world: &mut World,
    ) -> GroupHandle {
        use crate::ComponentIndex;

        let mut ty = type_ref_mut!(self, C);
        match &mut ty.storage {
            ComponentTypeStorage::MultipleGroups(groups) => {
                let index = groups.insert_with(|group_index| {
                    for (component_index, component) in storage.components.iter_with_index_mut() {
                        component.init(
                            ComponentHandle::new(
                                ComponentIndex(component_index),
                                C::IDENTIFIER,
                                GroupHandle(group_index),
                            ),
                            world,
                        )
                    }

                    storage
                });
                return GroupHandle(index);
            }
            _ => panic!("Component does not have ComponentStorage::Groups"),
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize<C: Component + serde::de::DeserializeOwned>(
        &mut self,
        data: Vec<u8>,
    ) {
        let deserialized: ComponentType<C> = bincode::deserialize(&data).unwrap();
        let mut ty = type_ref_mut!(self, C);
        *ty = deserialized;
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize<C: Component + serde::Serialize>(&self) -> Vec<u8> {
        bincode::serialize(&*type_ref!(self, C)).unwrap()
    }

    pub fn register_with_config<C: Component>(
        &mut self,
        groups: &GroupManager,
        config: ComponentConfig,
    ) {
        let mut globals = self.global.0.borrow_mut();
        match config.scope {
            ComponentScope::Scene => {
                if let Some(ty) = globals.get(&C::IDENTIFIER) {
                    assert!(
                        ty.is_none(),
                        "This component already exists as a global component!"
                    );
                } else {
                    globals.insert(C::IDENTIFIER, None);
                }
                if !self.types.contains_key(&C::IDENTIFIER) {
                    self.types.insert(
                        C::IDENTIFIER,
                        ComponentTypeScope::Scene(Box::new(RefCell::new(
                            ComponentType::<C>::with_config(config, groups),
                        ))),
                    );
                }
            }
            ComponentScope::Global => {
                if let Some(ty) = globals.get(&C::IDENTIFIER) {
                    if let Some(ty) = ty {
                        if !self.types.contains_key(&C::IDENTIFIER) {
                            self.types
                                .insert(C::IDENTIFIER, ComponentTypeScope::Global(ty.clone()));
                        }
                    } else {
                        panic!("This component already exists as a non global component!");
                    }
                } else {
                    globals.insert(
                        C::IDENTIFIER,
                        Some(Rc::new(RefCell::new(ComponentType::<C>::with_config(
                            config, groups,
                        )))),
                    );
                    let ty = globals[&C::IDENTIFIER].as_ref().unwrap();
                    if !self.types.contains_key(&C::IDENTIFIER) {
                        self.types
                            .insert(C::IDENTIFIER, ComponentTypeScope::Global(ty.clone()));
                    }
                }
            }
        }
        self.controllers.register::<C>(config);
    }

    pub(crate) fn instance_data(
        &self,
        handle: ComponentHandle,
        world: &World,
    ) -> Option<InstancePosition> {
        self.types
            .get(&handle.type_id())
            .unwrap()
            .ref_mut_raw()
            .camera_target(world, handle)
    }

    pub(crate) fn resource<'a, C: Component>(&'a self) -> &'a ComponentType<C> {
        return type_render!(self, C);
    }

    pub fn change_group<C: Component>(
        &mut self,
        component: ComponentHandle,
        new_group_handle: GroupHandle,
    ) -> Option<ComponentHandle> {
        let mut ty = type_ref_mut!(self, C);
        return ty.change_group(component, new_group_handle);
    }

    pub fn group_filter<'a>(&'a self, filter: GroupFilter<'a>) -> &'a [GroupHandle] {
        return group_filter!(self, filter).1;
    }

    #[inline]
    pub fn set_ref<'a, C: Component>(&'a self) -> ComponentSet<'a, C> {
        self.set_ref_of(GroupFilter::Active)
    }

    pub fn set_ref_of<'a, C: Component>(&'a self, filter: GroupFilter<'a>) -> ComponentSet<'a, C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        return ComponentSet::new(ty, groups);
    }

    #[inline]
    pub fn set_mut<'a, C: Component>(&'a mut self) -> ComponentSetMut<'a, C> {
        self.set_mut_of(GroupFilter::Active)
    }

    pub fn set_mut_of<'a, C: Component>(
        &'a mut self,
        filter: GroupFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    #[inline]
    pub fn set<'a, C: Component>(&'a self) -> ComponentSetMut<'a, C> {
        self.set_of(GroupFilter::Active)
    }

    pub fn set_of<'a, C: Component>(&'a self, filter: GroupFilter<'a>) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    pub fn index<C: Component>(&self, index: usize) -> Option<Ref<C>> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_mut<C: Component>(&mut self, index: usize) -> Option<RefMut<C>> {
        self.index_mut_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_of<C: Component>(&self, group: GroupHandle, index: usize) -> Option<Ref<C>> {
        let ty = type_ref!(self, C);
        Ref::filter_map(ty, |ty| ty.index(group, index)).ok()
    }

    pub fn index_mut_of<C: Component>(
        &mut self,
        group: GroupHandle,
        index: usize,
    ) -> Option<RefMut<C>> {
        let ty = type_ref_mut!(self, C);
        RefMut::filter_map(ty, |ty| ty.index_mut(group, index)).ok()
    }

    pub fn get<C: Component>(&self, handle: ComponentHandle) -> Option<Ref<C>> {
        let ty = type_ref!(self, C);
        Ref::filter_map(ty, |ty| ty.get(handle)).ok()
    }

    pub fn get_mut<C: Component>(&mut self, handle: ComponentHandle) -> Option<RefMut<C>> {
        let ty = type_ref_mut!(self, C);
        RefMut::filter_map(ty, |ty| ty.get_mut(handle)).ok()
    }

    pub fn remove<C: Component>(
        &mut self,
        world: &mut World,
        handle: ComponentHandle,
    ) -> Option<C> {
        let mut ty = type_ref_mut!(self, C);
        ty.remove(world, handle)
    }

    pub fn add_to<C: Component>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.add(world, group_handle, component)
    }

    pub fn add<C: Component>(&mut self, world: &mut World, component: C) -> ComponentHandle {
        self.add_to(world, GroupHandle::DEFAULT_GROUP, component)
    }

    #[inline]
    pub fn add_many<C: Component>(
        &mut self,
        world: &mut World,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.add_many_to(world, GroupHandle::DEFAULT_GROUP, components)
    }

    pub fn add_many_to<C: Component>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let mut ty = type_ref_mut!(self, C);
        ty.add_many(world, group_handle, components)
    }

    #[inline]
    pub fn add_with<C: Component>(
        &mut self,
        world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.add_with_to(world, GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to<C: Component>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.add_with(world, group_handle, create)
    }

    #[inline]
    pub fn remove_all<C: Component>(&mut self, world: &mut World) -> Vec<C> {
        self.remove_all_of(world, GroupFilter::All)
    }

    pub fn remove_all_of<C: Component>(
        &mut self,
        world: &mut World,
        filter: GroupFilter,
    ) -> Vec<C> {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.remove_all(world, groups)
    }

    #[inline]
    pub fn force_buffer<C: Component>(&mut self) {
        self.force_buffer_of::<C>(GroupFilter::All)
    }

    pub fn force_buffer_of<C: Component>(&mut self, filter: GroupFilter) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.force_buffer(groups)
    }

    #[inline]
    pub fn len<C: Component>(&self) -> usize {
        self.len_of::<C>(GroupFilter::All)
    }

    pub fn len_of<C: Component>(&self, filter: GroupFilter) -> usize {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.len(groups)
    }

    // #[inline]
    // pub fn iter<C: Component>(&self) -> impl DoubleEndedIterator<Item = &C> {
    //     self.iter_of::<C>(GroupFilter::Active)
    // }

    // pub fn iter_of<C: Component>(
    //     &self,
    //     filter: GroupFilter,
    // ) -> Box<dyn DoubleEndedIterator<Item = &'_ C> + '_> {
    //     let groups = group_filter!(self, filter).1;
    //     let ty = type_ref!(self, C);
    //     Ref::map(ty, |ty| ty.iter(groups))

    // }

    // #[inline]
    // pub fn iter_mut<C: Component>(&mut self) -> impl DoubleEndedIterator<Item = &mut C> {
    //     self.iter_mut_of::<C>(GroupFilter::Active)
    // }

    // pub fn iter_mut_of<C: Component>(
    //     &mut self,
    //     filter: GroupFilter,
    // ) -> impl DoubleEndedIterator<Item = &mut C> {
    //     let (check, groups) = group_filter!(self, filter);
    //     let mut ty = type_ref_mut!(self, C);
    //     ty.iter_mut(groups, check)
    // }

    // #[inline]
    // pub fn iter_with_handles<'a, C: Component>(
    //     &'a self,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
    //     self.iter_with_handles_of::<C>(GroupFilter::Active)
    // }

    // pub fn iter_with_handles_of<'a, C: Component>(
    //     &'a self,
    //     filter: GroupFilter<'a>,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
    //     let groups = group_filter!(self, filter).1;
    //     let ty = type_ref!(self, C);
    //     ty.iter_with_handles(groups)
    // }

    // #[inline]
    // pub fn iter_mut_with_handles<'a, C: Component>(
    //     &'a mut self,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
    //     self.iter_mut_with_handles_of::<C>(GroupFilter::Active)
    // }

    // pub fn iter_mut_with_handles_of<'a, C: Component>(
    //     &'a mut self,
    //     filter: GroupFilter<'a>,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
    //     let (check, groups) = group_filter!(self, filter);
    //     let mut ty = type_ref_mut!(self, C);
    //     ty.iter_mut_with_handles(groups, check)
    // }

    #[inline]
    pub fn for_each<C: Component>(&self, each: impl FnMut(&C)) {
        self.for_each_of(GroupFilter::Active, each)
    }

    pub fn for_each_of<C: Component>(&self, filter: GroupFilter, each: impl FnMut(&C)) {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.for_each(groups, each);
    }

    #[inline]
    pub fn for_each_mut<C: Component>(&mut self, each: impl FnMut(&mut C)) {
        self.for_each_mut_of(GroupFilter::Active, each)
    }

    #[inline]
    #[cfg(feature = "rayon")]
    pub fn par_for_each<C: Component + Send + Sync>(&self, each: impl Fn(&C) + Send + Sync) {
        self.par_for_each_of(GroupFilter::Active, each)
    }

    #[cfg(feature = "rayon")]
    pub fn par_for_each_of<C: Component + Send + Sync>(
        &self,
        filter: GroupFilter,
        each: impl Fn(&C) + Send + Sync,
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.par_for_each(groups, each);
    }

    pub fn buffer_for_each_mut<C: Component>(
        &mut self,
        world: &World,
        gpu: &Gpu,
        each: impl Fn(&mut C) + Send + Sync + Copy,
    ) {
        self.buffer_for_each_mut_of(world, gpu, GroupFilter::Active, each)
    }

    pub fn buffer_for_each_mut_of<C: Component>(
        &mut self,
        world: &World,
        gpu: &Gpu,
        filter: GroupFilter,
        each: impl Fn(&mut C) + Send + Sync + Copy,
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.buffer_for_each_mut(world, gpu, groups, each);
    }

    pub fn for_each_mut_of<C: Component>(&mut self, filter: GroupFilter, each: impl FnMut(&mut C)) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.for_each_mut(groups, each);
    }

    #[inline]
    #[cfg(feature = "rayon")]
    pub fn par_for_each_mut<C: Component + Send + Sync>(
        &mut self,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        self.par_for_each_mut_of(GroupFilter::Active, each)
    }

    #[cfg(feature = "rayon")]
    pub fn par_for_each_mut_of<C: Component + Send + Sync>(
        &mut self,
        filter: GroupFilter,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.par_for_each_mut(groups, each);
    }

    #[inline]
    pub fn retain<C: Component>(
        &mut self,
        world: &mut World,
        keep: impl FnMut(&mut C, &mut World) -> bool,
    ) {
        self.retain_of(world, GroupFilter::Active, keep)
    }

    pub fn retain_of<C: Component>(
        &mut self,
        world: &mut World,
        filter: GroupFilter,
        keep: impl FnMut(&mut C, &mut World) -> bool,
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.retain(world, groups, keep);
    }

    pub fn single<C: Component>(&self) -> Ref<C> {
        let ty = type_ref!(self, C);
        Ref::map(ty, |ty| ty.single())
    }

    pub fn single_mut<C: Component>(&mut self) -> RefMut<C> {
        let ty = type_ref_mut!(self, C);
        RefMut::map(ty, |ty| ty.single_mut())
    }

    pub fn single_ref<C: Component>(&self) -> RefMut<C> {
        let ty = type_ref_mut!(self, C);
        RefMut::map(ty, |ty| ty.single_mut())
    }

    pub fn try_single<C: Component>(&self) -> Option<Ref<C>> {
        let ty = type_ref!(self, C);
        Ref::filter_map(ty, |ty| ty.try_single()).ok()
    }

    pub fn try_single_mut<C: Component>(&mut self) -> Option<RefMut<C>> {
        let ty = type_ref_mut!(self, C);
        RefMut::filter_map(ty, |ty| ty.try_single_mut()).ok()
    }

    pub fn remove_single<C: Component>(&mut self, world: &mut World) -> Option<C> {
        let mut ty = type_ref_mut!(self, C);
        ty.remove_single(world)
    }

    pub fn set_single<C: Component>(&mut self, world: &mut World, new: C) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.set_single(world, new)
    }

    pub fn set_single_with<C: Component>(
        &mut self,
        world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.set_single_with(world, create)
    }
}
