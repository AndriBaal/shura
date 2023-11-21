use downcast_rs::{impl_downcast, Downcast};
use rustc_hash::FxHashMap;

use crate::{
    BufferConfig, CameraBuffer, CameraBuffer2D, Component, ComponentBufferManager,
    DefaultResources, Entity, EntityConfig, EntityHandle, EntityScope, EntitySet, EntitySetMut,
    EntityType, EntityTypeId, GlobalEntitys, Gpu, GroupHandle, Instance2D, InstanceBuffer,
    InstanceIndex, InstanceIndices, Mesh2D, Renderer, Scene, SystemManager, World, WorldCamera3D,
};

#[cfg(feature = "serde")]
use crate::{EntityTypeGroup, EntityTypeStorage};

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

pub(crate) trait EntityTypeImplementation: Downcast {
    fn add_group(&mut self);
    fn remove_group(&mut self, world: &mut World, handle: GroupHandle);
    fn buffer(
        &self,
        buffers: &mut ComponentBufferManager,
        world: &World,
        active_groups: &[GroupHandle],
    );
    fn entity_type_id(&self) -> EntityTypeId;
    fn config(&self) -> EntityConfig;

    #[cfg(all(feature = "serde", feature = "physics"))]
    fn deinit_non_serialized(&self, world: &mut World);
    #[cfg(feature = "serde")]
    fn remove_group_serialize(
        &mut self,
        world: &mut World,
        handle: GroupHandle,
    ) -> Option<Box<dyn std::any::Any>>;
}
impl_downcast!(EntityTypeImplementation);

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

const ALREADY_BORROWED: &'static str = "This type is already borrowed!";
fn no_type_error<E: Entity>() -> String {
    format!("The type '{}' first needs to be registered!", E::TYPE_NAME)
}

pub(crate) enum EntityTypeScope {
    Scene(Box<RefCell<dyn EntityTypeImplementation>>),
    Global(Rc<RefCell<dyn EntityTypeImplementation>>),
}

impl EntityTypeScope {
    fn ref_mut_raw(&self) -> RefMut<dyn EntityTypeImplementation> {
        match &self {
            EntityTypeScope::Scene(scene) => scene.try_borrow_mut().expect(ALREADY_BORROWED),
            EntityTypeScope::Global(global) => global.try_borrow_mut().expect(ALREADY_BORROWED),
        }
    }

    fn ref_raw(&self) -> Ref<dyn EntityTypeImplementation> {
        match &self {
            EntityTypeScope::Scene(scene) => scene.try_borrow().expect(ALREADY_BORROWED),
            EntityTypeScope::Global(global) => global.try_borrow().expect(ALREADY_BORROWED),
        }
    }

    fn _ref<E: Entity>(&self) -> Ref<EntityType<E>> {
        match &self {
            EntityTypeScope::Scene(scene) => {
                Ref::map(scene.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<EntityType<E>>().unwrap()
                })
            }
            EntityTypeScope::Global(global) => {
                Ref::map(global.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<EntityType<E>>().unwrap()
                })
            }
        }
    }

    fn ref_mut<E: Entity>(&self) -> RefMut<EntityType<E>> {
        match &self {
            EntityTypeScope::Scene(scene) => {
                RefMut::map(scene.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<EntityType<E>>().unwrap()
                })
            }
            EntityTypeScope::Global(global) => {
                RefMut::map(global.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<EntityType<E>>().unwrap()
                })
            }
        }
    }

    fn resource<E: Entity>(&self) -> &EntityType<E> {
        // This is safe, because we disallow .borrow_mut() with the ContextUse
        unsafe {
            match &self {
                EntityTypeScope::Scene(scene) => scene
                    .try_borrow_unguarded()
                    .unwrap()
                    .downcast_ref::<EntityType<E>>()
                    .unwrap(),
                EntityTypeScope::Global(global) => global
                    .try_borrow_unguarded()
                    .unwrap()
                    .downcast_ref::<EntityType<E>>()
                    .unwrap(),
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
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

pub struct RenderContext<'a> {
    entities: &'a EntityManager,
    components: &'a ComponentBufferManager,

    pub world_camera2d: &'a CameraBuffer2D,
    pub world_camera3d: &'a CameraBuffer<WorldCamera3D>,
    pub relative_camera: &'a CameraBuffer2D,
    pub relative_bottom_left_camera: &'a CameraBuffer2D,
    pub relative_bottom_right_camera: &'a CameraBuffer2D,
    pub relative_top_left_camera: &'a CameraBuffer2D,
    pub relative_top_right_camera: &'a CameraBuffer2D,
    pub unit_camera: &'a CameraBuffer2D,
    pub unit_mesh: &'a Mesh2D,
    pub centered_instance: &'a InstanceBuffer<Instance2D>,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        defaults: &'a DefaultResources,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        return (
            &scene.systems,
            Self {
                entities: &scene.entities,
                components: &scene.components,
                relative_camera: &defaults.relative_camera.0,
                relative_bottom_left_camera: &defaults.relative_bottom_left_camera.0,
                relative_bottom_right_camera: &defaults.relative_bottom_right_camera.0,
                relative_top_left_camera: &defaults.relative_top_left_camera.0,
                relative_top_right_camera: &defaults.relative_top_right_camera.0,
                unit_camera: &defaults.unit_camera.0,
                centered_instance: &defaults.centered_instance,
                unit_mesh: &defaults.unit_mesh,
                world_camera2d: &defaults.world_camera2d,
                world_camera3d: &defaults.world_camera3d,
            },
        );
    }

    #[inline]
    pub fn set<E: Entity>(&'a self) -> EntitySet<'a, E> {
        self.set_of(GroupFilter::Active)
    }

    pub fn set_of<E: Entity>(&'a self, filter: GroupFilter<'a>) -> EntitySet<'a, E> {
        let groups = group_filter!(self.entities, filter).1;
        let ty = type_ref!(self.entities, E);
        return EntitySet::new(ty, groups);
    }

    pub fn single<E: Entity>(&self) -> Ref<E> {
        let ty = type_ref!(self.entities, E);
        Ref::map(ty, |ty| ty.single())
    }

    pub fn try_single<E: Entity>(&self) -> Option<Ref<E>> {
        let ty = type_ref!(self.entities, E);
        Ref::filter_map(ty, |ty| ty.try_single()).ok()
    }

    // pub fn render_each<E: Entity>(
    //     &self,
    //     renderer: &mut Renderer<'a>,
    //     each: impl FnMut(
    //         &mut Renderer<'a>,
    //         &'a E,
    //         &'a InstanceBuffer<<E::Component as Component>::Instance>,
    //         InstanceIndex,
    //     ),
    // ) {
    //     let ty = type_render!(self.entities, E);
    //     ty.render_each(renderer, each)
    // }

    // pub fn render_single<E: Entity>(
    //     &self,
    //     renderer: &mut Renderer<'a>,
    //     each: impl FnOnce(
    //         &mut Renderer<'a>,
    //         &'a E,
    //         &'a InstanceBuffer<<E::Component as Component>::Instance>,
    //         InstanceIndex,
    //     ),
    // ) {
    //     let ty = type_render!(self.entities, E);
    //     ty.render_single(renderer, each)
    // }

    pub fn render_all<I: crate::Instance>(
        &self,
        renderer: &mut Renderer<'a>,
        name: &'static str,
        all: impl Fn(&mut Renderer<'a>, &'a InstanceBuffer<I>, InstanceIndices),
    ) {
        let buffer = self
            .components
            .get::<I>(name)
            .expect(&format!("Component {name} is not registered!"))
            .buffer();

        if buffer.instance_amount() != 0 {
            (all)(renderer, buffer, buffer.instances());
        }
    }
}

pub struct EntityManager {
    pub(super) active_groups: Vec<GroupHandle>,
    pub(super) all_groups: Vec<GroupHandle>,
    pub(crate) types: FxHashMap<EntityTypeId, EntityTypeScope>,
}

impl EntityManager {
    pub(crate) fn empty() -> Self {
        return Self {
            all_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            active_groups: Vec::from_iter([GroupHandle::DEFAULT_GROUP]),
            types: Default::default(),
        };
    }

    pub(crate) fn new(
        global: &GlobalEntitys,
        entities: Vec<Box<RefCell<dyn EntityTypeImplementation>>>,
    ) -> Self {
        let mut manager = Self::empty();
        manager.init(global, entities);
        return manager;
    }

    pub(crate) fn init(
        &mut self,
        global: &GlobalEntitys,
        entities: Vec<Box<RefCell<dyn EntityTypeImplementation>>>,
    ) {
        let mut globals = global.0.borrow_mut();
        for entity in entities {
            let config = entity.borrow().config();
            let id = entity.borrow().entity_type_id();
            match config.scope {
                EntityScope::Scene => {
                    if let Some(ty) = globals.get(&id) {
                        assert!(
                            ty.is_none(),
                            "This entity already exists as a global entity!"
                        );
                    } else {
                        globals.insert(id, None);
                    }
                    self.types.insert(id, EntityTypeScope::Scene(entity.into()));
                }
                EntityScope::Global => {
                    if let Some(ty) = globals.get(&id) {
                        if let Some(ty) = ty {
                            if !self.types.contains_key(&id) {
                                self.types.insert(id, EntityTypeScope::Global(ty.clone()));
                            }
                        } else {
                            panic!("This entity already exists as a non global entity!");
                        }
                    } else {
                        globals.insert(id, Some(entity.into()));
                        let ty = globals[&id].as_ref().unwrap();
                        if !self.types.contains_key(&id) {
                            self.types.insert(id, EntityTypeScope::Global(ty.clone()));
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn buffer(&mut self, buffers: &mut ComponentBufferManager, world: &World) {
        for ty in &self.types {
            let ty = ty.1.ref_raw();
            ty.buffer(buffers, world, &self.active_groups);
        }
    }

    pub(crate) fn types_mut(
        &mut self,
    ) -> impl Iterator<Item = RefMut<'_, dyn EntityTypeImplementation>> {
        self.types.values_mut().map(|r| r.ref_mut_raw())
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize_group<E: Entity + serde::de::DeserializeOwned>(
        &mut self,
        mut storage: EntityTypeGroup<E>,
        world: &mut World,
    ) -> GroupHandle {
        use crate::EntityIndex;

        let mut ty = type_ref_mut!(self, E);
        match &mut ty.storage {
            EntityTypeStorage::MultipleGroups(groups) => {
                let index = groups.insert_with(|group_index| {
                    for (entity_index, entity) in storage.entities.iter_mut_with_index() {
                        entity.init(
                            EntityHandle::new(
                                EntityIndex(entity_index),
                                E::IDENTIFIER,
                                GroupHandle(group_index),
                            ),
                            world,
                        )
                    }

                    storage
                });
                return GroupHandle(index);
            }
            _ => panic!("Entity does not have EntityStorage::Groups"),
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize<E: Entity + serde::Serialize>(&self) -> Vec<u8> {
        bincode::serialize(&*type_ref!(self, E)).unwrap()
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
    pub fn set_ref<'a, E: Entity>(&'a self) -> EntitySet<'a, E> {
        self.set_ref_of(GroupFilter::Active)
    }

    pub fn set_ref_of<'a, E: Entity>(&'a self, filter: GroupFilter<'a>) -> EntitySet<'a, E> {
        let groups = group_filter!(self, filter).1;
        let ty = type_ref!(self, E);
        return EntitySet::new(ty, groups);
    }

    #[inline]
    pub fn set_mut<'a, E: Entity>(&'a mut self) -> EntitySetMut<'a, E> {
        self.set_mut_of(GroupFilter::Active)
    }

    pub fn set_mut_of<'a, E: Entity>(&'a mut self, filter: GroupFilter<'a>) -> EntitySetMut<'a, E> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, E);
        return EntitySetMut::new(ty, groups, check);
    }

    #[inline]
    pub fn set<'a, E: Entity>(&'a self) -> EntitySetMut<'a, E> {
        self.set_of(GroupFilter::Active)
    }

    pub fn set_of<'a, E: Entity>(&'a self, filter: GroupFilter<'a>) -> EntitySetMut<'a, E> {
        let (check, groups) = group_filter!(self, filter);
        let ty = type_ref_mut!(self, E);
        return EntitySetMut::new(ty, groups, check);
    }

    pub fn single_ref<E: Entity>(&self) -> Ref<E> {
        let ty = type_ref!(self, E);
        Ref::map(ty, |ty| ty.single())
    }

    pub fn single_mut<E: Entity>(&mut self) -> RefMut<E> {
        let ty = type_ref_mut!(self, E);
        RefMut::map(ty, |ty| ty.single_mut())
    }

    pub fn single<E: Entity>(&self) -> RefMut<E> {
        let ty = type_ref_mut!(self, E);
        RefMut::map(ty, |ty| ty.single_mut())
    }

    pub fn try_single<E: Entity>(&self) -> Option<RefMut<E>> {
        let ty = type_ref_mut!(self, E);
        RefMut::filter_map(ty, |ty: &mut EntityType<E>| ty.try_single_mut()).ok()
    }

    pub fn try_single_mut<E: Entity>(&mut self) -> Option<RefMut<E>> {
        let ty = type_ref_mut!(self, E);
        RefMut::filter_map(ty, |ty: &mut EntityType<E>| ty.try_single_mut()).ok()
    }

    pub fn try_single_ref<E: Entity>(&self) -> Option<Ref<E>> {
        let ty = type_ref!(self, E);
        Ref::filter_map(ty, |ty| ty.try_single()).ok()
    }

    pub fn add_to<E: Entity>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        entity: E,
    ) -> EntityHandle {
        let mut ty = type_ref_mut!(self, E);
        ty.add(world, group_handle, entity)
    }

    pub fn add<E: Entity>(&mut self, world: &mut World, entity: E) -> EntityHandle {
        self.add_to(world, GroupHandle::DEFAULT_GROUP, entity)
    }

    #[inline]
    pub fn add_many<E: Entity>(
        &mut self,
        world: &mut World,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        self.add_many_to(world, GroupHandle::DEFAULT_GROUP, entities)
    }

    pub fn add_many_to<E: Entity>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        entities: impl IntoIterator<Item = E>,
    ) -> Vec<EntityHandle> {
        let mut ty = type_ref_mut!(self, E);
        ty.add_many(world, group_handle, entities)
    }

    #[inline]
    pub fn add_with<E: Entity>(
        &mut self,
        world: &mut World,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        self.add_with_to(world, GroupHandle::DEFAULT_GROUP, create)
    }

    pub fn add_with_to<E: Entity>(
        &mut self,
        world: &mut World,
        group_handle: GroupHandle,
        create: impl FnOnce(EntityHandle) -> E,
    ) -> EntityHandle {
        let mut ty = type_ref_mut!(self, E);
        ty.add_with(world, group_handle, create)
    }
}
