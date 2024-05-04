use crate::entity::{EntityId, EntityType};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

pub(crate) type GlobalEntities = Mutex<FxHashMap<EntityId, Option<Rc<RefCell<dyn EntityType>>>>>;
