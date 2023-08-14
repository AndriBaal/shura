use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

use crate::{
    ComponentBuffer, ComponentConfig, ComponentController, ComponentHandle, ComponentScope,
    ComponentSet, ComponentSetMut, ComponentSetResource, ComponentType, ComponentTypeId,
    ContextUse, ControllerManager, GlobalComponents, Gpu, GroupHandle, GroupManager,
    InstancePosition, World,
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

    fn get_ref<C: ComponentController + 'static>(&self) -> Ref<ComponentType<C>> {
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

    fn get_ref_mut<C: ComponentController + 'static>(&self) -> RefMut<ComponentType<C>> {
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

    fn get_resource<C: ComponentController + 'static>(&self) -> &ComponentType<C> {
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

        // #[cfg(not(feature = "rayon"))]
        // for (buffer, index) in self.controllers.buffers() {
        //     let ty = &self.types[index];
        //     ty.get_ref_mut()
        //         .buffer(world, *buffer, &self.active_groups, &gpu);
        // }

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

    pub fn register<C: ComponentController + ComponentBuffer + 'static>(
        &mut self,
        groups: &GroupManager,
    ) {
        self.register_with_config::<C>(groups, C::CONFIG);
    }

    pub fn register_with_config<C: ComponentController + ComponentBuffer + 'static>(
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

    pub(crate) fn resource_of<'a, C: ComponentController + 'static>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetResource<'a, C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_render!(self, C);
        return ComponentSetResource::new(ty, groups);
    }

    #[inline]
    pub fn set_ref<'a, C: ComponentController + 'static>(&'a self) -> ComponentSet<'a, C> {
        self.set_ref_of(ComponentFilter::Active)
    }

    pub fn set_ref_of<'a, C: ComponentController + 'static>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSet<'a, C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        return ComponentSet::new(ty, groups);
    }

    #[inline]
    pub fn set_mut<'a, C: ComponentController + ComponentBuffer + 'static>(
        &'a mut self,
    ) -> ComponentSetMut<'a, C> {
        self.set_mut_of(ComponentFilter::Active)
    }

    pub fn set_mut_of<'a, C: ComponentController + ComponentBuffer + 'static>(
        &'a mut self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }

    #[inline]
    pub fn set<'a, C: ComponentController + 'static + ComponentBuffer>(
        &'a self,
    ) -> ComponentSetMut<'a, C> {
        self.set_of(ComponentFilter::Active)
    }

    pub fn set_of<'a, C: ComponentController + 'static + ComponentBuffer>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }
}
