use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use rustc_hash::FxHashMap;

#[cfg(feature = "serde")]
use crate::entity::EntityGroupHandle;
use crate::{
    component::Component,
    entity::{
        Entities, Entity, EntityGroupManager, EntityId, EntityIdentifier, EntityType,
        GroupedEntities, SingleEntity,
    },
    graphics::RenderGroupManager,
    physics::World,
};

use super::{EntityHandle, GlobalEntities};

fn already_borrowed<E: EntityIdentifier>() -> String {
    format!(
        "The entity type {} is already mutably borrowed!",
        E::TYPE_NAME
    )
}

fn wrong_type<E: EntityIdentifier>() -> String {
    format!("Wrong entity storage for {}!", E::TYPE_NAME)
}

fn no_type_error<E: EntityIdentifier>() -> String {
    format!("The type '{}' is not registered!", E::TYPE_NAME)
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntityScope {
    #[default]
    Scene,
    Global,
}

pub(crate) enum EntityTypeScope {
    Scene(Box<RefCell<dyn EntityType>>),
    Global(Rc<RefCell<dyn EntityType>>),
}

impl EntityTypeScope {
    fn ref_mut_dyn(&self) -> RefMut<dyn EntityType> {
        match &self {
            EntityTypeScope::Scene(scene) => scene
                .try_borrow_mut()
                .expect("Type already borrowed!"),
            EntityTypeScope::Global(global) => global
                .try_borrow_mut()
                .expect("Type already borrowed!"),
        }
    }

    fn ref_dyn(&self) -> Ref<dyn EntityType> {
        match &self {
            EntityTypeScope::Scene(scene) => scene
                .try_borrow()
                .expect("Type already borrowed!"),
            EntityTypeScope::Global(global) => global
                .try_borrow()
                .expect("Type already borrowed!"),
        }
    }

    fn _ref<ET: EntityType>(&self) -> Ref<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => Ref::map(
                scene
                    .try_borrow()
                    .unwrap_or_else(|_| panic!("{}", already_borrowed::<ET::Entity>())),
                |ty| {
                    ty.downcast_ref::<ET>()
                        .unwrap_or_else(|| panic!("{}", &wrong_type::<ET::Entity>()))
                },
            ),
            EntityTypeScope::Global(global) => Ref::map(
                global
                    .try_borrow()
                    .unwrap_or_else(|_| panic!("{}", already_borrowed::<ET::Entity>())),
                |ty| {
                    ty.downcast_ref::<ET>()
                        .unwrap_or_else(|| panic!("{}", &wrong_type::<ET::Entity>()))
                },
            ),
        }
    }

    fn ref_mut<ET: EntityType>(&self) -> RefMut<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => RefMut::map(
                scene
                    .try_borrow_mut()
                    .unwrap_or_else(|_| panic!("{}", already_borrowed::<ET::Entity>())),
                |ty| {
                    ty.downcast_mut::<ET>()
                        .unwrap_or_else(|| panic!("{}", &wrong_type::<ET::Entity>()))
                },
            ),
            EntityTypeScope::Global(global) => RefMut::map(
                global
                    .try_borrow_mut()
                    .unwrap_or_else(|_| panic!("{}", already_borrowed::<ET::Entity>())),
                |ty| {
                    ty.downcast_mut::<ET>()
                        .unwrap_or_else(|| panic!("{}", &wrong_type::<ET::Entity>()))
                },
            ),
        }
    }
}

type TypeMap = FxHashMap<EntityId, EntityTypeScope>;

pub struct EntityManager {
    pub(crate) types: TypeMap,
    pub(crate) new_types: Vec<Box<dyn FnOnce(&mut Self, &GlobalEntities)>>,
    pub(crate) components: FxHashMap<&'static str, Vec<EntityId>>,
}

impl EntityManager {
    pub(crate) fn new() -> Self {
        Self {
            types: Default::default(),
            new_types: Default::default(),
            components: Default::default(),
        }
    }

    pub(crate) fn add_type<ET: EntityType>(&mut self, scope: EntityTypeScope) {
        let previous = self.types.insert(ET::Entity::IDENTIFIER, scope);
        assert!(previous.is_none(), "Entity already defined!");
        for name in ET::Entity::tags() {
            self.components
                .entry(name)
                .or_default()
                .push(ET::Entity::IDENTIFIER);
        }
    }

    pub fn components_each(&self, tag: &'static str, each: impl Fn(EntityHandle, &dyn Component)) {
        if let Some(type_ids) = self.components.get(tag) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let ty = ty.ref_dyn();
                for (handle, entity) in ty.dyn_iter() {
                    if let Some(collection) = entity.component(tag) {
                        each(handle, collection);
                    }
                }
            }
        }
    }

    pub fn components_each_mut(
        &self,
        tag: &'static str,
        each: impl Fn(EntityHandle, &mut dyn Component),
    ) {
        if let Some(type_ids) = self.components.get(tag) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let mut ty = ty.ref_mut_dyn();
                for (handle, entity) in ty.dyn_iter_mut() {
                    if let Some(collection) = entity.component_mut(tag) {
                        each(handle, collection);
                    }
                }
            }
        }
    }

    pub fn entities_for_component(
        &self,
        tag: &'static str,
        each: impl Fn(EntityHandle, &dyn Entity),
    ) {
        if let Some(type_ids) = self.components.get(tag) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let ty = ty.ref_dyn();
                for (handle, entity) in ty.dyn_iter() {
                    each(handle, entity);
                }
            }
        }
    }

    pub fn entities_for_component_mut(
        &self,
        tag: &'static str,
        each: impl Fn(EntityHandle, &mut dyn Entity),
    ) {
        if let Some(type_ids) = self.components.get(tag) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let mut ty = ty.ref_mut_dyn();
                for (handle, entity) in ty.dyn_iter_mut() {
                    each(handle, entity);
                }
            }
        }
    }

    pub fn retain_entities_for_component(
        &self,
        world: &mut World,
        tag: &'static str,
        keep: impl Fn(&mut dyn Entity, &mut World) -> bool,
    ) {
        if let Some(type_ids) = self.components.get(tag) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let mut ty = ty.ref_mut_dyn();
                ty.dyn_retain(world, &keep);
            }
        }
    }

    pub fn component_mapping(&self) -> &FxHashMap<&'static str, Vec<EntityId>> {
        &self.components
    }

    pub fn entities_with_component(&self, name: &'static str) -> Option<&Vec<EntityId>> {
        return self.components.get(name);
    }

    pub fn register_entity<ET: EntityType>(&mut self, scope: EntityScope, ty: ET) {
        let id = ET::Entity::IDENTIFIER;
        if self.types.contains_key(&id) {
            panic!("Entity {} already defined!", ET::Entity::TYPE_NAME);
        }

        if TypeId::of::<ET>() == TypeId::of::<GroupedEntities<ET>>() && scope == EntityScope::Global
        {
            panic!(
                "Global component can not be stored in groups because groups are scene specific!"
            );
        }

        self.new_types
            .push(Box::new(move |entities: &mut EntityManager, globals| {
                let mut globals = globals.lock().unwrap();
                match scope {
                    EntityScope::Scene => {
                        if let Some(ty) = globals.get(&id) {
                            assert!(
                                ty.is_none(),
                                "The entity {} already exists as a global entity!",
                                ET::Entity::TYPE_NAME
                            );
                        } else {
                            globals.insert(id, None);
                        }
                        entities.add_type::<ET>(EntityTypeScope::Scene(Box::new(RefCell::new(ty))));
                    }
                    EntityScope::Global => {
                        if let Some(ty) = globals.get(&id) {
                            if let Some(ty) = ty {
                                entities.add_type::<ET>(EntityTypeScope::Global(ty.clone()));
                            } else {
                                panic!("This entity already exists as a non global entity!");
                            }
                        } else {
                            globals.insert(id, Some(Rc::new(RefCell::new(ty))));
                            let ty = globals[&id].as_ref().unwrap();
                            entities.add_type::<ET>(EntityTypeScope::Global(ty.clone()));
                        }
                    }
                }
            }));
    }

    pub fn apply_registered(&mut self, globals: &GlobalEntities) {
        for new in self.new_types.drain(..).collect::<Vec<_>>() {
            (new)(self, globals)
        }
    }

    pub fn buffer(
        &mut self,
        buffers: &mut RenderGroupManager,
        groups: &EntityGroupManager,
        world: &World,
    ) {
        for ty in &self.types {
            let ty = ty.1.ref_dyn();
            ty.buffer(buffers, groups, world);
        }
    }

    pub fn types(&mut self) -> impl Iterator<Item = Ref<'_, dyn EntityType>> {
        self.types.values_mut().map(|r| r.ref_dyn())
    }

    pub fn types_mut(&mut self) -> impl Iterator<Item = RefMut<'_, dyn EntityType>> {
        self.types.values_mut().map(|r| r.ref_mut_dyn())
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialize_group<ET: EntityType + Default>(
        &mut self,
        group: &EntityGroupHandle,
        entity_type: ET,
        world: &mut World,
    ) {
        let mut groups = self.group_mut::<ET>();
        let group = groups.get_group_mut(group).unwrap();
        *group = entity_type;
        for (handle, entity) in group.dyn_iter_mut() {
            entity.init(handle, world);
        }
    }

    pub fn exists<E: EntityIdentifier>(&self) -> bool {
        return self.types.contains_key(&E::IDENTIFIER);
    }

    #[cfg(feature = "serde")]
    pub fn serialize<ET: EntityType + serde::Serialize>(&self) -> Vec<u8> {
        bincode::serialize(
            self.type_raw(ET::Entity::IDENTIFIER)
                .downcast_ref::<ET>()
                .unwrap(),
        )
        .unwrap()
    }

    pub fn type_raw_mut(&self, type_id: EntityId) -> RefMut<dyn EntityType> {
        self.types
            .get(&type_id)
            .expect("Cannot find type!")
            .ref_mut_dyn()
    }

    pub fn type_raw(&self, type_id: EntityId) -> Ref<dyn EntityType> {
        self.types
            .get(&type_id)
            .expect("Cannot find type!")
            .ref_dyn()
    }

    pub fn single_mut<E: EntityIdentifier>(&self) -> RefMut<SingleEntity<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            .ref_mut()
    }

    pub fn single<E: EntityIdentifier>(&self) -> Ref<SingleEntity<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            ._ref()
    }

    pub fn multiple_mut<E: EntityIdentifier>(&self) -> RefMut<Entities<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            .ref_mut()
    }

    pub fn multiple<E: EntityIdentifier>(&self) -> Ref<Entities<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            ._ref()
    }

    pub fn group_mut<ET: EntityType + Default>(&self) -> RefMut<GroupedEntities<ET>> {
        self.types
            .get(&ET::Entity::IDENTIFIER)
            .expect(&no_type_error::<ET::Entity>())
            .ref_mut()
    }

    pub fn group<ET: EntityType + Default>(&self) -> Ref<GroupedEntities<ET>> {
        self.types
            .get(&ET::Entity::IDENTIFIER)
            .expect(&no_type_error::<ET::Entity>())
            ._ref()
    }

    pub fn try_type_raw_mut(&self, type_id: EntityId) -> Option<RefMut<dyn EntityType>> {
        if let Some(ty) = self.types.get(&type_id) {
            return Some(ty.ref_mut_dyn());
        }
        return None;
    }

    pub fn try_type_raw(&self, type_id: EntityId) -> Option<Ref<dyn EntityType>> {
        if let Some(ty) = self.types.get(&type_id) {
            return Some(ty.ref_dyn());
        }
        return None;
    }

    pub fn try_single_mut<E: EntityIdentifier>(&self) -> Option<RefMut<SingleEntity<E>>> {
        if let Some(ty) = self.types.get(&E::IDENTIFIER) {
            return Some(ty.ref_mut());
        }
        return None;
    }

    pub fn try_single<E: EntityIdentifier>(&self) -> Option<Ref<SingleEntity<E>>> {
        if let Some(ty) = self.types.get(&E::IDENTIFIER) {
            return Some(ty._ref());
        }
        return None;
    }

    pub fn try_multiple_mut<E: EntityIdentifier>(&self) -> Option<RefMut<Entities<E>>> {
        if let Some(ty) = self.types.get(&E::IDENTIFIER) {
            return Some(ty.ref_mut());
        }
        return None;
    }

    pub fn try_multiple<E: EntityIdentifier>(&self) -> Option<Ref<Entities<E>>> {
        if let Some(ty) = self.types.get(&E::IDENTIFIER) {
            return Some(ty._ref());
        }
        return None;
    }

    pub fn try_group_mut<ET: EntityType + Default>(&self) -> Option<RefMut<GroupedEntities<ET>>> {
        if let Some(ty) = self.types.get(&ET::Entity::IDENTIFIER) {
            return Some(ty.ref_mut());
        }
        return None;
    }

    pub fn try_group<ET: EntityType + Default>(&self) -> Option<Ref<GroupedEntities<ET>>> {
        if let Some(ty) = self.types.get(&ET::Entity::IDENTIFIER) {
            return Some(ty._ref());
        }
        return None;
    }
}
