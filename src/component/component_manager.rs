use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

use crate::{
    ComponentConfig, ComponentController, ComponentHandle, ComponentScope, ComponentSet,
    ComponentSetMut, ComponentSetResource, ComponentType, ComponentTypeId, ContextUse,
    ControllerManager, GlobalComponents, Gpu, GroupHandle, GroupManager, InstancePosition, World,
};
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub(crate) trait ComponentTypeImplementation: Downcast {
    fn add_group(&mut self);
    fn remove_group(&mut self, world: &mut World, handle: GroupHandle);
    fn camera_target(&self, world: &World, handle: ComponentHandle) -> Option<InstancePosition>;
    fn buffer(&mut self, world: &World, active: &[GroupHandle], gpu: &Gpu);
}
impl_downcast!(ComponentTypeImplementation);

#[macro_export]
/// Register multiple components at once
macro_rules! register {
    ($ctx: expr, [$($C:ty),* $(,)?]) => {
        {
            $(
                $ctx.components.register::<$C>($ctx.groups);
            )*
        }
    };
}

macro_rules! group_filter {
    ($self:ident, $filter: expr) => {
        match $filter {
            ComponentFilter::All => (false, &$self.all_groups[..]),
            ComponentFilter::Active => (false, &$self.active_groups[..]),
            ComponentFilter::Custom(h) => (true, h),
        }
    };
}

macro_rules! type_ref {
    ($self:ident, $C: ident) => {{
        let ty = $self
            .types
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            .get_ref::<$C>();
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
            .get_ref_mut::<$C>();
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
            .get_resource::<$C>();
        ty
    }};
}

const ALREADY_BORROWED: &'static str = "This type is already borrowed!";
fn no_type_error<C: ComponentController>() -> String {
    format!("The type '{}' first needs to be registered!", C::TYPE_NAME)
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) enum ComponentTypeScope {
    Scene(Box<RefCell<dyn ComponentTypeImplementation>>),
    Global(Rc<RefCell<dyn ComponentTypeImplementation>>),
}

impl ComponentTypeScope {
    fn get_ref_mut_raw(&self) -> RefMut<dyn ComponentTypeImplementation> {
        match &self {
            ComponentTypeScope::Scene(scene) => scene.try_borrow_mut().expect(ALREADY_BORROWED),
            ComponentTypeScope::Global(global) => global.try_borrow_mut().expect(ALREADY_BORROWED),
        }
    }

    fn get_ref<C: ComponentController>(&self) -> Ref<ComponentType<C>> {
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

    fn get_ref_mut<C: ComponentController>(&self) -> RefMut<ComponentType<C>> {
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

    fn get_resource<C: ComponentController>(&self) -> &ComponentType<C> {
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
pub enum ComponentFilter<'a> {
    All,
    Active,
    Custom(&'a [GroupHandle]),
}

impl<'a> Default for ComponentFilter<'a> {
    fn default() -> Self {
        return ComponentFilter::Active;
    }
}

impl ComponentFilter<'static> {
    pub const DEFAULT_GROUP: Self = ComponentFilter::Custom(&[GroupHandle::DEFAULT_GROUP]);
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Access to the component system
pub struct ComponentManager {
    types: FxHashMap<ComponentTypeId, ComponentTypeScope>,
    global: GlobalComponents,
    context_use: ContextUse,
    pub(super) active_groups: Vec<GroupHandle>,
    pub(super) all_groups: Vec<GroupHandle>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) controllers: Rc<ControllerManager>,
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
                ty.get_ref_mut_raw()
                    .buffer(world, &self.active_groups, &gpu);
            });
        }

        #[cfg(not(feature = "rayon"))]
        for ty in self.controllers.buffers() {
            let ty = &self.types[ty];
            ty.get_ref_mut_raw()
                .buffer(world, &self.active_groups, &gpu);
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn type_ref<C: ComponentController>(
        &self,
    ) -> impl Deref<Target = ComponentType<C>> + '_ {
        type_ref!(self, C)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn type_mut<C: ComponentController>(
        &mut self,
    ) -> impl DerefMut<Target = ComponentType<C>> + '_ {
        type_mut!(self, C)
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
        self.types.values_mut().map(|r| r.get_ref_mut_raw())
    }

    pub fn register<C: ComponentController>(&mut self, groups: &GroupManager) {
        self.register_with_config::<C>(groups, C::CONFIG);
    }

    pub fn register_with_config<C: ComponentController>(
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
            .get_ref_mut_raw()
            .camera_target(world, handle)
    }

    pub(crate) fn resource_of<'a, C: ComponentController>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetResource<'a, C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_render!(self, C);
        return ComponentSetResource::new(ty, groups);
    }

    #[inline]
    pub fn set_ref<'a, C: ComponentController>(&'a self) -> ComponentSet<'a, C> {
        self.set_ref_of(ComponentFilter::Active)
    }

    pub fn set_ref_of<'a, C: ComponentController>(
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
        let ty = type_ref_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    #[inline]
    pub fn set<'a, C: ComponentController>(&'a self) -> ComponentSetMut<'a, C> {
        self.set_of(ComponentFilter::Active)
    }

    pub fn set_of<'a, C: ComponentController>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    pub fn index<C: ComponentController>(&self, index: usize) -> Option<Ref<C>> {
        self.index_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_mut<C: ComponentController>(&mut self, index: usize) -> Option<RefMut<C>> {
        self.index_mut_of(GroupHandle::DEFAULT_GROUP, index)
    }

    pub fn index_of<C: ComponentController>(
        &self,
        group: GroupHandle,
        index: usize,
    ) -> Option<Ref<C>> {
        let ty = type_ref!(self, C);
        Ref::filter_map(ty, |ty| ty.index(group, index)).ok()
    }

    pub fn index_mut_of<C: ComponentController>(
        &mut self,
        group: GroupHandle,
        index: usize,
    ) -> Option<RefMut<C>> {
        let ty = type_ref_mut!(self, C);
        RefMut::filter_map(ty, |ty| ty.index_mut(group, index)).ok()
    }

    pub fn get<C: ComponentController>(&self, handle: ComponentHandle) -> Option<Ref<C>> {
        let ty = type_ref!(self, C);
        Ref::filter_map(ty, |ty| ty.get(handle)).ok()
    }

    pub fn get_mut<C: ComponentController>(
        &mut self,
        handle: ComponentHandle,
    ) -> Option<RefMut<C>> {
        let ty = type_ref_mut!(self, C);
        RefMut::filter_map(ty, |ty| ty.get_mut(handle)).ok()
    }

    pub fn remove<C: ComponentController>(
        &mut self,
        world: &mut World,
        handle: ComponentHandle,
    ) -> Option<C> {
        let mut ty = type_ref_mut!(self, C);
        ty.remove(world, handle)
    }

    pub fn add_to<C: ComponentController>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        component: C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.add(world, group_handle, component)
    }

    pub fn add<C: ComponentController>(
        &mut self,
        world: &mut World,
        component: C,
    ) -> ComponentHandle {
        self.add_to(world, GroupHandle::DEFAULT_GROUP, component)
    }

    #[inline]
    pub fn add_many<C: ComponentController>(
        &mut self,
        world: &mut World,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        self.add_many_to(world, GroupHandle::DEFAULT_GROUP, components)
    }

    pub fn add_many_to<C: ComponentController>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        components: impl IntoIterator<Item = C>,
    ) -> Vec<ComponentHandle> {
        let mut ty = type_ref_mut!(self, C);
        ty.add_many(world, group_handle, components)
    }

    #[inline]
    pub fn add_with<C: ComponentController>(
        &mut self,
        world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        self.add_with_to(world, GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to<C: ComponentController>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.add_with(world, group_handle, create)
    }

    #[inline]
    pub fn remove_all<C: ComponentController>(&mut self, world: &mut World) -> Vec<C> {
        self.remove_all_of(world, ComponentFilter::All)
    }

    pub fn remove_all_of<C: ComponentController>(
        &mut self,
        world: &mut World,
        filter: ComponentFilter,
    ) -> Vec<C> {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.remove_all(world, groups)
    }

    #[inline]
    pub fn force_buffer<C: ComponentController>(&mut self) {
        self.force_buffer_of::<C>(ComponentFilter::All)
    }

    pub fn force_buffer_of<C: ComponentController>(&mut self, filter: ComponentFilter) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
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

    // #[inline]
    // pub fn iter<C: ComponentController>(&self) -> impl DoubleEndedIterator<Item = &C> {
    //     self.iter_of::<C>(ComponentFilter::Active)
    // }

    // pub fn iter_of<C: ComponentController>(
    //     &self,
    //     filter: ComponentFilter,
    // ) -> Box<dyn DoubleEndedIterator<Item = &'_ C> + '_> {
    //     let groups = group_filter!(self, filter).1;
    //     let ty = type_ref!(self, C);
    //     Ref::map(ty, |ty| ty.iter(groups))

    // }

    // #[inline]
    // pub fn iter_mut<C: ComponentController>(&mut self) -> impl DoubleEndedIterator<Item = &mut C> {
    //     self.iter_mut_of::<C>(ComponentFilter::Active)
    // }

    // pub fn iter_mut_of<C: ComponentController>(
    //     &mut self,
    //     filter: ComponentFilter,
    // ) -> impl DoubleEndedIterator<Item = &mut C> {
    //     let (check, groups) = group_filter!(self, filter);
    //     let mut ty = type_ref_mut!(self, C);
    //     ty.iter_mut(groups, check)
    // }

    // #[inline]
    // pub fn iter_with_handles<'a, C: ComponentController>(
    //     &'a self,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
    //     self.iter_with_handles_of::<C>(ComponentFilter::Active)
    // }

    // pub fn iter_with_handles_of<'a, C: ComponentController>(
    //     &'a self,
    //     filter: ComponentFilter<'a>,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
    //     let groups = group_filter!(self, filter).1;
    //     let ty = type_ref!(self, C);
    //     ty.iter_with_handles(groups)
    // }

    // #[inline]
    // pub fn iter_mut_with_handles<'a, C: ComponentController>(
    //     &'a mut self,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
    //     self.iter_mut_with_handles_of::<C>(ComponentFilter::Active)
    // }

    // pub fn iter_mut_with_handles_of<'a, C: ComponentController>(
    //     &'a mut self,
    //     filter: ComponentFilter<'a>,
    // ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a mut C)> {
    //     let (check, groups) = group_filter!(self, filter);
    //     let mut ty = type_ref_mut!(self, C);
    //     ty.iter_mut_with_handles(groups, check)
    // }

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
    #[cfg(feature = "rayon")]
    pub fn par_for_each<C: ComponentController + Send + Sync>(
        &self,
        each: impl Fn(&C) + Send + Sync,
    ) {
        self.par_for_each_of(ComponentFilter::Active, each)
    }

    #[cfg(feature = "rayon")]
    pub fn par_for_each_of<C: ComponentController + Send + Sync>(
        &self,
        filter: ComponentFilter,
        each: impl Fn(&C) + Send + Sync,
    ) {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        ty.par_for_each(groups, each);
    }

    #[cfg(feature = "rayon")]
    pub fn buffer_for_each_mut<C: ComponentController>(
        &mut self,
        world: &World,
        gpu: &Gpu,
        each: impl Fn(&mut C) + Send + Sync + Copy,
    ) {
        self.buffer_for_each_mut_of(world, gpu, ComponentFilter::Active, each)
    }

    #[cfg(feature = "rayon")]
    pub fn buffer_for_each_mut_of<C: ComponentController>(
        &mut self,
        world: &World,
        gpu: &Gpu,
        filter: ComponentFilter,
        each: impl Fn(&mut C) + Send + Sync + Copy,
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.buffer_for_each_mut(world, gpu, groups, each);
    }

    pub fn for_each_mut_of<C: ComponentController>(
        &mut self,
        filter: ComponentFilter,
        each: impl FnMut(&mut C),
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.for_each_mut(groups, each);
    }

    #[inline]
    #[cfg(feature = "rayon")]
    pub fn par_for_each_mut<C: ComponentController + Send + Sync>(
        &mut self,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        self.par_for_each_mut_of(ComponentFilter::Active, each)
    }

    #[cfg(feature = "rayon")]
    pub fn par_for_each_mut_of<C: ComponentController + Send + Sync>(
        &mut self,
        filter: ComponentFilter,
        each: impl Fn(&mut C) + Send + Sync,
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.par_for_each_mut(groups, each);
    }

    #[inline]
    pub fn retain<C: ComponentController>(
        &mut self,
        world: &mut World,
        keep: impl FnMut(&mut C, &mut World) -> bool,
        #[cfg(not(feature = "physics"))] keep: impl FnMut(&mut C) -> bool,
    ) {
        self.retain_of(world, ComponentFilter::Active, keep)
    }

    pub fn retain_of<C: ComponentController>(
        &mut self,
        world: &mut World,
        filter: ComponentFilter,
        keep: impl FnMut(&mut C, &mut World) -> bool,
        #[cfg(not(feature = "physics"))] keep: impl FnMut(&mut C) -> bool,
    ) {
        let groups = group_filter!(self, filter).1;
        let mut ty = type_ref_mut!(self, C);
        ty.retain(world, groups, keep);
    }

    pub fn single<C: ComponentController>(&self) -> Ref<C> {
        let ty = type_ref!(self, C);
        Ref::map(ty, |ty| ty.single())
    }

    pub fn single_mut<C: ComponentController>(&mut self) -> RefMut<C> {
        let ty = type_ref_mut!(self, C);
        RefMut::map(ty, |ty| ty.single_mut())
    }

    pub fn try_single<C: ComponentController>(&self) -> Option<Ref<C>> {
        let ty = type_ref!(self, C);
        Ref::filter_map(ty, |ty| ty.try_single()).ok()
    }

    pub fn try_single_mut<C: ComponentController>(&mut self) -> Option<RefMut<C>> {
        let ty = type_ref_mut!(self, C);
        RefMut::filter_map(ty, |ty| ty.try_single_mut()).ok()
    }

    pub fn remove_single<C: ComponentController>(&mut self, world: &mut World) -> Option<C> {
        let mut ty = type_ref_mut!(self, C);
        ty.remove_single(world)
    }

    pub fn set_single<C: ComponentController>(
        &mut self,
        world: &mut World,
        new: C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.set_single(world, new)
    }

    pub fn set_single_with<C: ComponentController>(
        &mut self,
        world: &mut World,
        create: impl FnOnce(ComponentHandle) -> C,
    ) -> ComponentHandle {
        let mut ty = type_ref_mut!(self, C);
        ty.set_single_with(world, create)
    }
}
