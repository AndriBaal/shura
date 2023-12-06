use rustc_hash::FxHashMap;

use crate::{
    ComponentBufferManager, Entities, EntityIdentifier, EntityType, EntityTypeId, GlobalEntities,
    GroupManager, GroupedEntities, SingleEntity, World,
};

#[cfg(feature = "serde")]
use crate::GroupHandle;

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

const ALREADY_BORROWED: &str = "This type is already borrowed!";
fn no_type_error<E: EntityIdentifier>() -> String {
    format!("The type '{}' first needs to be registered!", E::TYPE_NAME)
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
    fn ref_mut_raw(&self) -> RefMut<dyn EntityType> {
        match &self {
            EntityTypeScope::Scene(scene) => scene.try_borrow_mut().expect(ALREADY_BORROWED),
            EntityTypeScope::Global(global) => global.try_borrow_mut().expect(ALREADY_BORROWED),
        }
    }

    fn ref_raw(&self) -> Ref<dyn EntityType> {
        match &self {
            EntityTypeScope::Scene(scene) => scene.try_borrow().expect(ALREADY_BORROWED),
            EntityTypeScope::Global(global) => global.try_borrow().expect(ALREADY_BORROWED),
        }
    }

    fn _ref<ET: EntityType>(&self) -> Ref<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => {
                Ref::map(scene.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<ET>().unwrap()
                })
            }
            EntityTypeScope::Global(global) => {
                Ref::map(global.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<ET>().unwrap()
                })
            }
        }
    }

    fn ref_mut<ET: EntityType>(&self) -> RefMut<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => {
                RefMut::map(scene.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<ET>().unwrap()
                })
            }
            EntityTypeScope::Global(global) => {
                RefMut::map(global.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<ET>().unwrap()
                })
            }
        }
    }
}

pub struct EntityManager {
    pub(crate) types: FxHashMap<EntityTypeId, EntityTypeScope>,
}

impl EntityManager {
    pub(crate) fn empty() -> Self {
        Self {
            types: Default::default(),
        }
    }

    pub(crate) fn new(
        global: &GlobalEntities,
        entities: Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>,
    ) -> Self {
        let mut manager = Self::empty();
        manager.init(global, entities);
        manager
    }

    pub(crate) fn init(
        &mut self,
        global: &GlobalEntities,
        entities: Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>,
    ) {
        let mut globals = global.0.borrow_mut();
        for (scope, entity) in entities {
            let id = entity.borrow().entity_type_id();
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
                    self.types.insert(id, EntityTypeScope::Scene(entity));
                }
                EntityScope::Global => {
                    if let Some(ty) = globals.get(&id) {
                        if let Some(ty) = ty {
                            self.types
                                .entry(id)
                                .or_insert_with(|| EntityTypeScope::Global(ty.clone()));
                        } else {
                            panic!("This entity already exists as a non global entity!");
                        }
                    } else {
                        globals.insert(id, Some(entity.into()));
                        let ty = globals[&id].as_ref().unwrap();
                        self.types
                            .entry(id)
                            .or_insert_with(|| EntityTypeScope::Global(ty.clone()));
                    }
                }
            }
        }
    }

    pub(crate) fn buffer(
        &mut self,
        buffers: &mut ComponentBufferManager,
        groups: &GroupManager,
        world: &World,
    ) {
        for ty in &self.types {
            let ty = ty.1.ref_raw();
            Ref::clone(&ty).buffer(buffers, groups, world);
        }
    }

    pub(crate) fn types_mut(&mut self) -> impl Iterator<Item = RefMut<'_, dyn EntityType>> {
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
                GroupHandle(index)
            }
            _ => panic!("Entity does not have EntityStorage::Groups"),
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialize<E: EntityIdentifier + serde::Serialize>(&self) -> Vec<u8> {
        bincode::serialize(&*self.type_raw::<E>()).unwrap()
    }

    pub fn type_raw<E: EntityIdentifier>(&self) -> RefMut<dyn EntityType> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            .ref_mut_raw()
    }

    pub fn type_raw_ref<E: EntityIdentifier>(&self) -> Ref<dyn EntityType> {
        self.types
            .get(&E::IDENTIFIER)
            .expect(&no_type_error::<E>())
            .ref_raw()
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
