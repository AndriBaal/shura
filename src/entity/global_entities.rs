use crate::entity::{EntityType, EntityTypeId};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

// The Option<> is here to keep track of entities, that have already been added to scenes and therefore
// can not be registered as a global entities.
pub(crate) type GlobalEntities =
    Rc<RefCell<FxHashMap<EntityTypeId, Option<Rc<RefCell<dyn EntityType>>>>>>;

pub(crate) const GLOBAL_ENTITIES: std::cell::OnceCell<GlobalEntities> = std::cell::OnceCell::new();
