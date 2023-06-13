use rustc_hash::FxHashMap;

use crate::{StateDerive, StateIdentifier, StateTypeId};

#[derive(Default)]
/// Manager of [States](crate::State)
pub struct StateManager {
    states: FxHashMap<StateTypeId, Box<dyn StateDerive>>,
}

impl StateManager {
    pub fn try_get<T: StateDerive + StateIdentifier>(&self) -> Option<&T> {
        self.states
            .get(&T::IDENTIFIER)
            .and_then(|s| s.downcast_ref::<T>())
    }

    pub fn try_get_mut<T: StateDerive + StateIdentifier>(&mut self) -> Option<&mut T> {
        self.states
            .get_mut(&T::IDENTIFIER)
            .and_then(|s| s.downcast_mut::<T>())
    }

    pub fn try_remove<T: StateDerive + StateIdentifier>(&mut self) -> Option<Box<T>> {
        self.states
            .remove(&T::IDENTIFIER)
            .and_then(|s| s.downcast::<T>().ok())
    }

    pub fn insert<T: StateDerive + StateIdentifier>(&mut self, state: T) {
        self.states.insert(T::IDENTIFIER, Box::new(state));
    }

    pub fn contains<T: StateDerive + StateIdentifier>(&self) -> bool {
        self.states.contains_key(&T::IDENTIFIER)
    }

    pub fn remove<T: StateDerive + StateIdentifier>(&mut self) -> Box<T> {
        self.try_remove().unwrap()
    }

    pub fn get<T: StateDerive + StateIdentifier>(&self) -> &T {
        self.try_get().unwrap()
    }

    pub fn get_mut<T: StateDerive + StateIdentifier>(&mut self) -> &mut T {
        self.try_get_mut().unwrap()
    }
}
