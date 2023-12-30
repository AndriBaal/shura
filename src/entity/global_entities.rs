use crate::entity::{EntityType, EntityTypeId};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc, sync::Mutex};

pub(crate) type GlobalEntities =
    Mutex<FxHashMap<EntityTypeId, Option<Rc<RefCell<dyn EntityType>>>>>;
