use rustc_hash::FxHashMap;

use crate::{
    entity::{
        Entities, Entity, EntityGroupManager, EntityIdentifier, EntityType, EntityTypeId,
        GroupedEntities, SingleEntity,
    },
    graphics::RenderGroupManager,
    physics::World,
    prelude::Component,
};

#[cfg(feature = "serde")]
use crate::entity::EntityGroupHandle;

use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use super::{EntityHandle, GlobalEntities};

const ALREADY_BORROWED: &str = "This type is already borrowed!";
const WRONG_TYPE: &str = "Wrong type";
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
            EntityTypeScope::Scene(scene) => scene.try_borrow_mut().expect(ALREADY_BORROWED),
            EntityTypeScope::Global(global) => global.try_borrow_mut().expect(ALREADY_BORROWED),
        }
    }

    fn ref_dyn(&self) -> Ref<dyn EntityType> {
        match &self {
            EntityTypeScope::Scene(scene) => scene.try_borrow().expect(ALREADY_BORROWED),
            EntityTypeScope::Global(global) => global.try_borrow().expect(ALREADY_BORROWED),
        }
    }

    fn _ref<ET: EntityType>(&self) -> Ref<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => Ref::map(
                scene
                    .try_borrow()
                    .unwrap_or_else(|_| panic!("Type {} already borrowed", ET::Entity::TYPE_NAME)),
                |ty| ty.downcast_ref::<ET>().expect(WRONG_TYPE),
            ),
            EntityTypeScope::Global(global) => Ref::map(
                global
                    .try_borrow()
                    .unwrap_or_else(|_| panic!("Type {} already borrowed", ET::Entity::TYPE_NAME)),
                |ty| ty.downcast_ref::<ET>().expect(WRONG_TYPE),
            ),
        }
    }

    fn ref_mut<ET: EntityType>(&self) -> RefMut<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => RefMut::map(
                scene
                    .try_borrow_mut()
                    .unwrap_or_else(|_| panic!("Type {} already borrowed", ET::Entity::TYPE_NAME)),
                |ty| ty.downcast_mut::<ET>().expect(WRONG_TYPE),
            ),
            EntityTypeScope::Global(global) => RefMut::map(
                global
                    .try_borrow_mut()
                    .unwrap_or_else(|_| panic!("Type {} already borrowed", ET::Entity::TYPE_NAME)),
                |ty| ty.downcast_mut::<ET>().expect(WRONG_TYPE),
            ),
        }
    }
}

type TypeMap = FxHashMap<EntityTypeId, EntityTypeScope>;
pub struct EntityManager {
    pub(crate) types: TypeMap,
    pub(crate) new_types: Vec<Box<dyn FnOnce(&mut Self, &GlobalEntities)>>,
    pub(crate) components: FxHashMap<&'static str, Vec<EntityTypeId>>,
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
        for name in ET::Entity::named_components() {
            self.components
                .entry(name)
                .or_default()
                .push(ET::Entity::IDENTIFIER);
        }
    }

    pub fn components_each(
        &self,
        name: &'static str,
        each: impl Fn(EntityHandle, &dyn Entity, &dyn Component),
    ) {
        if let Some(type_ids) = self.components.get(name) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let ty = ty.ref_dyn();
                for (handle, entity) in ty.entities() {
                    let component_collection = entity.component_collection(name).unwrap();
                    for collection in component_collection {
                        for component in collection.components() {
                            each(handle, entity, component);
                        }
                    }
                }
            }
        }
    }

    pub fn components_each_mut(
        &self,
        name: &'static str,
        each: impl Fn(EntityHandle, &mut dyn Component),
    ) {
        if let Some(type_ids) = self.components.get(name) {
            for type_id in type_ids {
                let ty = self.types.get(type_id).unwrap();
                let mut ty = ty.ref_mut_dyn();
                for (handle, entity) in ty.entities_mut() {
                    let component_collection = entity.component_collection_mut(name).unwrap();
                    for collection in component_collection {
                        for component in collection.components_mut() {
                            each(handle, component);
                        }
                    }
                }
            }
        }
    }

    pub fn component_mapping(&self) -> &FxHashMap<&'static str, Vec<EntityTypeId>> {
        &self.components
    }

    pub fn entities_with_component(&self, name: &'static str) -> Option<&Vec<EntityTypeId>> {
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
                                "This entity already exists as a global entity!"
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
            Ref::clone(&ty).buffer(buffers, groups, world);
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
        group: EntityGroupHandle,
        storage: ET,
        world: &mut World,
    ) {
        let mut groups = self.group::<ET>();
        let group = groups.get_group_mut(group).unwrap();
        *group = storage;
        for (handle, entity) in group.entities_mut() {
            entity.init(handle, world);
        }
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

    pub fn type_raw(&self, type_id: EntityTypeId) -> RefMut<dyn EntityType> {
        self.types
            .get(&type_id)
            .expect("Cannot find type!")
            .ref_mut_dyn()
    }

    pub fn type_raw_ref(&self, type_id: EntityTypeId) -> Ref<dyn EntityType> {
        self.types
            .get(&type_id)
            .expect("Cannot find type!")
            .ref_dyn()
    }

    pub fn single<E: EntityIdentifier>(&self) -> RefMut<SingleEntity<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            .ref_mut()
    }

    pub fn single_ref<E: EntityIdentifier>(&self) -> Ref<SingleEntity<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            ._ref()
    }

    pub fn multiple<E: EntityIdentifier>(&self) -> RefMut<Entities<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            .ref_mut()
    }

    pub fn multiple_ref<E: EntityIdentifier>(&self) -> Ref<Entities<E>> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            ._ref()
    }

    pub fn group<ET: EntityType + Default>(&self) -> RefMut<GroupedEntities<ET>> {
        self.types
            .get(&ET::Entity::IDENTIFIER)
            .expect(&no_type_error::<ET::Entity>())
            .ref_mut()
    }

    pub fn group_ref<ET: EntityType + Default>(&self) -> Ref<GroupedEntities<ET>> {
        self.types
            .get(&ET::Entity::IDENTIFIER)
            .expect(&no_type_error::<ET::Entity>())
            ._ref()
    }
}
