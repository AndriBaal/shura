use std::{cell::RefCell, rc::Rc};
use rustc_hash::FxHashMap;
use crate::entity::{EntityType, EntityTypeId};

pub(crate) type InnerGlobalEntities =
    Rc<RefCell<FxHashMap<EntityTypeId, Option<Rc<RefCell<dyn EntityType>>>>>>;

// The Option<> is here to keep track of entities, that have already been added to scenes and therefore
// can not be registered as a global entities.
pub struct GlobalEntities(pub(crate) InnerGlobalEntities);
