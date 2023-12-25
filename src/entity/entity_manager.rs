use rustc_hash::FxHashMap;

use crate::{
    entity::{
        Entities, EntityIdentifier, EntityType, EntityTypeId, GroupManager, GroupedEntities,
        SingleEntity,
    },
    graphics::ComponentBufferManager,
    physics::World,
};

#[cfg(feature = "serde")]
use crate::entity::GroupHandle;

use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use super::GLOBAL_ENTITIES;

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
                    ty.downcast_ref::<ET>().expect(WRONG_TYPE)
                })
            }
            EntityTypeScope::Global(global) => {
                Ref::map(global.try_borrow().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_ref::<ET>().expect(WRONG_TYPE)
                })
            }
        }
    }

    fn ref_mut<ET: EntityType>(&self) -> RefMut<ET> {
        match &self {
            EntityTypeScope::Scene(scene) => {
                RefMut::map(scene.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<ET>().expect(WRONG_TYPE)
                })
            }
            EntityTypeScope::Global(global) => {
                RefMut::map(global.try_borrow_mut().expect(ALREADY_BORROWED), |ty| {
                    ty.downcast_mut::<ET>().expect(WRONG_TYPE)
                })
            }
        }
    }
}

pub struct EntityManager {
    pub(crate) types: FxHashMap<EntityTypeId, EntityTypeScope>,
}

impl EntityManager {
    pub(crate) fn new() -> Self {
        Self {
            types: Default::default(),
        }
    }

    pub fn register_entity<ET: EntityType>(&mut self, scope: EntityScope, ty: ET) {
        let rc = GLOBAL_ENTITIES.get_or_init(|| Default::default()).clone();
        let mut globals = rc.borrow_mut();
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
                self.types
                    .insert(id, EntityTypeScope::Scene(Box::new(RefCell::new(ty))));
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
                    globals.insert(id, Some(Rc::new(RefCell::new(ty))));
                    let ty = globals[&id].as_ref().unwrap();
                    self.types
                        .entry(id)
                        .or_insert_with(|| EntityTypeScope::Global(ty.clone()));
                }
            }
        }
    }

    pub fn buffer(
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
    pub(crate) fn deserialize_group<ET: EntityType + Default>(
        &mut self,
        group: GroupHandle,
        storage: ET,
        world: &mut World,
    ) {
        let mut groups = self.group::<ET>();
        let group = groups.get_group_mut(group).unwrap();
        *group = storage;
        for (handle, entity) in group.iter_dyn() {
            entity.init(handle, world);
        }
    }

    #[cfg(feature = "serde")]
    pub fn serialize<ET: EntityType + serde::Serialize>(&self) -> Vec<u8> {
        bincode::serialize(&*self.type_raw::<ET::Entity>().downcast_ref::<ET>().unwrap()).unwrap()
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
