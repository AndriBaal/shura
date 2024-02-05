use crate::entity::{EntityStorage, EntityId};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc, sync::Mutex};

pub(crate) type GlobalEntities =
    Mutex<FxHashMap<EntityId, Option<Rc<RefCell<dyn EntityStorage>>>>>;
