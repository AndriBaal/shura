use rustc_hash::FxHashMap;

#[cfg(feature = "physics")]
use crate::physics::World;
use crate::{
    ComponentBuffer, ComponentConfig, ComponentController, ComponentHandle, ComponentSet,
    ComponentSetMut, ComponentType, ComponentTypeId, ContextUse, ControllerManager, Gpu,
    GroupHandle, GroupManager, InstanceData, InstanceIndex, InstanceIndices, RenderCamera,
    Renderer,
};
use std::{
    cell::{RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[macro_export]
/// Register multiple components at once
macro_rules! register {
    ($components: expr, $groups: expr, [$($C:ty),* $(,)?]) => {
        {
            $(
                $components.register::<$C>($groups);
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
            .borrow();
        ty
    }};
}

macro_rules! type_mut {
    ($self:ident, $C: ident) => {{
        let ty = $self
            .types
            .get_mut(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            .borrow_mut();
        ty
    }};
}

macro_rules! type_render {
    ($self:ident, $C: ident) => {
        unsafe {
            let ty = $self
                .types
                .get(&$C::IDENTIFIER)
                .expect(&no_type_error::<$C>())
                .try_borrow_unguarded()
                .unwrap();
            ty
        }
    };
}

fn no_type_error<C: ComponentController>() -> String {
    format!("The type '{}' first needs to be registered!", C::TYPE_NAME)
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
    types: FxHashMap<ComponentTypeId, RefCell<ComponentType>>,
    context_use: ContextUse,
    pub(super) active_groups: Vec<GroupHandle>,
    pub(super) all_groups: Vec<GroupHandle>,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) controllers: Rc<ControllerManager>,
}

impl ComponentManager {
    pub(crate) fn new() -> Self {
        Self {
            types: Default::default(),
            all_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            active_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            controllers: Rc::new(ControllerManager::new()),
            context_use: ContextUse::Update,
        }
    }

    pub(crate) fn with_use(&mut self, context_use: ContextUse) -> &mut Self {
        self.context_use = context_use;
        self
    }

    pub(crate) fn buffer(&mut self, #[cfg(feature = "physics")] world: &World, gpu: &Gpu) {
        #[cfg(feature = "rayon")]
        // This is safe here because we dont't expand the map and we don't access the same map entry twice
        unsafe {
            struct UnsafeWrapper<'a>(&'a FxHashMap<ComponentTypeId, RefCell<ComponentType>>);
            impl<'a> UnsafeWrapper<'a> {
                pub unsafe fn get(&self, type_id: &ComponentTypeId) -> &mut ComponentType {
                    let ptr = &self.0[type_id] as *const _ as *mut ComponentType;
                    let ty = ptr.as_mut().unwrap();
                    return ty;
                }
            }
            unsafe impl<'a> Send for UnsafeWrapper<'a> {}
            unsafe impl<'a> Sync for UnsafeWrapper<'a> {}
            let wrapper = UnsafeWrapper(&self.types);

            use rayon::prelude::*;
            self.controllers
                .buffers()
                .par_iter()
                .for_each(|(buffer, type_id)| {
                    let ty = wrapper.get(type_id);
                    (*ty).buffer(
                        #[cfg(feature = "physics")]
                        world,
                        *buffer,
                        &self.active_groups,
                        &gpu,
                    );
                });
        }

        #[cfg(not(feature = "rayon"))]
        for (buffer, index) in self.controllers.buffers() {
            let ty = &self.types[index];
            ty.borrow_mut().buffer(
                #[cfg(feature = "physics")]
                world,
                *buffer,
                &self.active_groups,
                &gpu,
            );
        }
    }

    pub(crate) fn type_render<C: ComponentController>(
        &self,
    ) -> &ComponentType {
        type_render!(self, C)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn type_ref<C: ComponentController>(
        &self,
    ) -> impl Deref<Target = ComponentType> + '_ {
        type_ref!(self, C)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn type_mut<C: ComponentController>(
        &mut self,
    ) -> impl DerefMut<Target = ComponentType> + '_ {
        type_mut!(self, C)
    }

    pub(crate) fn types_mut(&mut self) -> impl Iterator<Item = RefMut<'_, ComponentType>> {
        self.types.values_mut().map(|r| r.borrow_mut())
    }

    pub fn register<C: ComponentController + ComponentBuffer>(&mut self, groups: &GroupManager) {
        self.register_with_config::<C>(groups, C::CONFIG);
    }

    pub fn register_with_config<C: ComponentController + ComponentBuffer>(
        &mut self,
        groups: &GroupManager,
        config: ComponentConfig,
    ) {
        if !self.types.contains_key(&C::IDENTIFIER) {
            self.types.insert(
                C::IDENTIFIER,
                RefCell::new(ComponentType::with_config::<C>(config, groups)),
            );
        }
        self.controllers.register::<C>(config);
    }

    pub fn active_groups(&self) -> &[GroupHandle] {
        &self.active_groups
    }

    pub fn all_groups(&self) -> &[GroupHandle] {
        &self.all_groups
    }

    pub(crate) fn instance_data(
        &self,
        handle: ComponentHandle,
        #[cfg(feature = "physics")] world: &World,
    ) -> Option<InstanceData> {
        self.types
            .get(&handle.type_id())
            .unwrap()
            .borrow()
            .get_boxed(handle)
            .map(|c| c.base().instance(#[cfg(feature = "physics")] world))
    }

    #[inline]
    pub fn get<'a, C: ComponentController>(&'a self) -> ComponentSet<'a, C> {
        self.get_of(ComponentFilter::Active)
    }

    pub fn get_of<'a, C: ComponentController>(
        &'a self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSet<'a, C> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, C);
        return ComponentSet::new(ty, groups);
    }

    #[inline]
    pub fn get_mut<'a, C: ComponentController + ComponentBuffer>(
        &'a mut self,
    ) -> ComponentSetMut<'a, C> {
        self.get_mut_of(ComponentFilter::Active)
    }

    pub fn get_mut_of<'a, C: ComponentController + ComponentBuffer>(
        &'a mut self,
        filter: ComponentFilter<'a>,
    ) -> ComponentSetMut<'a, C> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_mut!(self, C);
        return ComponentSetMut::new(ty, groups, check);
    }
}
