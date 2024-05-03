use crate::entity::{EntityId, EntityType};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};
use parking_lot::Mutex;

pub(crate) type GlobalEntities = Mutex<FxHashMap<EntityId, Option<Rc<RefCell<dyn EntityType>>>>>;
