use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

use crate::{
    Camera2D, Component, ComponentConfig, ComponentHandle, ComponentScope, ComponentSet,
    ComponentSetMut, ComponentType, ComponentTypeId, DefaultResources, GlobalComponents, Gpu,
    GroupHandle, Instance2D, InstanceBuffer, InstanceHandler, InstanceIndex, InstanceIndices,
    Model2D, Renderer, Scene, SystemManager, World, WorldCamera2D, BufferOperation,
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
    fn buffer(&mut self, world: &World, gpu: &Gpu, active: &[GroupHandle]);
    fn component_type_id(&self) -> ComponentTypeId;
    fn config(&self) -> ComponentConfig;

    #[cfg(all(feature = "serde", feature = "physics"))]
    fn deinit_non_serialized(&self, world: &mut World);
    #[cfg(feature = "serde")]
    fn remove_group_serialize(
        &mut self,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<Box<dyn std::any::Any>>;
}
impl_downcast!(ComponentTypeImplementation);

macro_rules! group_filter {
    ($self:expr, $filter: expr) => {
        match $filter {
            GroupFilter::All => (false, &$self.all_groups[..]),
            GroupFilter::Active => (false, &$self.active_groups[..]),
            GroupFilter::Custom(h) => (true, h),
        }
    };
}

macro_rules! type_ref {
    ($self:expr, $C: ident) => {{
        let ty = $self
            .types
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            ._ref::<$C>();
        ty
    }};
}

macro_rules! type_ref_mut {
    ($self:expr, $C: ident) => {{
        let ty = $self
            .types
            .get(&$C::IDENTIFIER)
            .expect(&no_type_error::<$C>())
            .ref_mut::<$C>();
        ty
    }};
}

macro_rules! type_render {
    ($self:expr, $C: ident) => {{
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

pub struct ComponentResources<'a> {
    components: &'a ComponentManager,

    pub world_camera: &'a WorldCamera2D,
    pub relative_camera: &'a Camera2D,
    pub relative_bottom_left_camera: &'a Camera2D,
    pub relative_bottom_right_camera: &'a Camera2D,
    pub relative_top_left_camera: &'a Camera2D,
    pub relative_top_right_camera: &'a Camera2D,
    pub unit_camera: &'a Camera2D,
    pub unit_model: &'a Model2D,
    pub centered_instance: &'a InstanceBuffer<Instance2D>,
}

impl<'a> ComponentResources<'a> {
    pub(crate) fn new(
        defaults: &'a DefaultResources,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        return (
            &scene.systems,
            Self {
                components: &scene.components,
                relative_camera: &defaults.relative_camera,
                relative_bottom_left_camera: &defaults.relative_bottom_left_camera,
                relative_bottom_right_camera: &defaults.relative_bottom_right_camera,
                relative_top_left_camera: &defaults.relative_top_left_camera,
                relative_top_right_camera: &defaults.relative_top_right_camera,
                unit_camera: &defaults.unit_camera,
                centered_instance: &defaults.centered_instance,
                unit_model: &defaults.unit_model,
                world_camera: &scene.world_camera,
            },
        );
    }

    #[inline]
    pub fn set<C: Component>(&'a self) -> ComponentSet<'a, C> {
        self.set_of(GroupFilter::Active)
    }

    pub fn set_of<C: Component>(&'a self, filter: GroupFilter<'a>) -> ComponentSet<'a, C> {
        let groups = group_filter!(self.components, filter).1;
        let ty = type_ref!(self.components, C);
        return ComponentSet::new(ty, groups);
    }

    pub fn single<C: Component>(&self) -> Ref<C> {
        let ty = type_ref!(self.components, C);
        Ref::map(ty, |ty| ty.single())
    }

    pub fn render_each<C: Component>(
        &self,
        renderer: &mut Renderer<'a>,
        each: impl FnMut(
            &mut Renderer<'a>,
            &'a C,
            &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
            InstanceIndex,
        ),
    ) {
        let ty = type_render!(self.components, C);
        ty.render_each(renderer, each)
    }

    pub fn render_single<C: Component>(
        &self,
        renderer: &mut Renderer<'a>,
        each: impl FnOnce(
            &mut Renderer<'a>,
            &'a C,
            &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
            InstanceIndex,
        ),
    ) {
        let ty = type_render!(self.components, C);
        ty.render_single(renderer, each)
    }

    pub fn render_all<C: Component>(
        &self,
        renderer: &mut Renderer<'a>,
        all: impl FnMut(
            &mut Renderer<'a>,
            &'a InstanceBuffer<<C::InstanceHandler as InstanceHandler>::Instance>,
            InstanceIndices,
        ),
    ) {
        let ty = type_render!(self.components, C);
        ty.render_all(renderer, all)
    }
}

/// Access to the component system
pub struct ComponentManager {
    pub(super) active_groups: Vec<GroupHandle>,
    pub(super) all_groups: Vec<GroupHandle>,
    pub(crate) types: FxHashMap<ComponentTypeId, ComponentTypeScope>,
}

impl ComponentManager {
    pub(crate) fn empty() -> Self {
        return Self {
            all_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            active_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            types: Default::default(),
        };
    }

    pub(crate) fn new(
        global: &GlobalComponents,
        components: Vec<Box<RefCell<dyn ComponentTypeImplementation>>>,
    ) -> Self {
        let mut manager = Self::empty();
        manager.init(global, components);
        return manager;
    }

    pub(crate) fn init(
        &mut self,
        global: &GlobalComponents,
        components: Vec<Box<RefCell<dyn ComponentTypeImplementation>>>,
    ) {
        let mut globals = global.0.borrow_mut();
        for component in components {
            let config = component.borrow().config();
            let id = component.borrow().component_type_id();
            match config.scope {
                ComponentScope::Scene => {
                    if let Some(ty) = globals.get(&id) {
                        assert!(
                            ty.is_none(),
                            "This component already exists as a global component!"
                        );
                    } else {
                        globals.insert(id, None);
                    }
                    self.types
                        .insert(id, ComponentTypeScope::Scene(component.into()));
                }
                ComponentScope::Global => {
                    if let Some(ty) = globals.get(&id) {
                        if let Some(ty) = ty {
                            if !self.types.contains_key(&id) {
                                self.types
                                    .insert(id, ComponentTypeScope::Global(ty.clone()));
                            }
                        } else {
                            panic!("This component already exists as a non global component!");
                        }
                    } else {
                        globals.insert(id, Some(component.into()));
                        let ty = globals[&id].as_ref().unwrap();
                        if !self.types.contains_key(&id) {
                            self.types
                                .insert(id, ComponentTypeScope::Global(ty.clone()));
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn buffer(&mut self, world: &World, gpu: &Gpu) {
        for ty in &self.types {
            let mut ty = ty.1.ref_mut_raw();
            if ty.config().buffer != BufferOperation::Never {
                ty.buffer(world, gpu, &self.active_groups);
            }
        }
    }

    pub(crate) fn types_mut(
        &mut self,
    ) -> impl Iterator<Item = RefMut<'_, dyn ComponentTypeImplementation>> {
        self.types.values_mut().map(|r| r.ref_mut_raw())
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
                    for (component_index, component) in storage.components.iter_mut_with_index() {
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
    pub(crate) fn serialize<C: Component + serde::Serialize>(&self) -> Vec<u8> {
        bincode::serialize(&*type_ref!(self, C)).unwrap()
    }

    pub fn active_groups(&self) -> &[GroupHandle] {
        &self.active_groups
    }

    pub fn all_groups(&self) -> &[GroupHandle] {
        &self.all_groups
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

    pub fn single_ref<C: Component>(&self) -> Ref<C> {
        let ty = type_ref!(self, C);
        Ref::map(ty, |ty| ty.single())
    }

    pub fn single_mut<C: Component>(&mut self) -> RefMut<C> {
        let ty = type_ref_mut!(self, C);
        RefMut::map(ty, |ty| ty.single_mut())
    }

    pub fn single<C: Component>(&self) -> RefMut<C> {
        let ty = type_ref_mut!(self, C);
        RefMut::map(ty, |ty| ty.single_mut())
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
}
