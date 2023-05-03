use crate::{State, StateTypeId};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::BTreeMap;

#[allow(unused_variables)]
pub trait GlobalStateController: Downcast {
    fn winit_event(&mut self, winit_event: &winit::event::Event<()>) {}
}
impl_downcast!(GlobalStateController);

#[derive(Default)]
pub struct GlobalStateManager {
    states: BTreeMap<(i16, StateTypeId), Box<dyn GlobalStateController>>,
}

impl GlobalStateManager {
    pub fn try_get<T: GlobalStateController + State>(&self) -> Option<&T> {
        self.states
            .get(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast_ref::<T>())
    }

    pub fn try_get_mut<T: GlobalStateController + State>(&mut self) -> Option<&mut T> {
        self.states
            .get_mut(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast_mut::<T>())
    }

    pub fn try_remove<T: GlobalStateController + State>(&mut self) -> Option<Box<T>> {
        self.states
            .remove(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast::<T>().ok())
    }

    pub fn insert<T: GlobalStateController + State>(&mut self, state: T) {
        self.states
            .insert((T::PRIORITY, T::IDENTIFIER), Box::new(state));
    }

    pub fn contains<T: GlobalStateController + State>(&self) -> bool {
        self.states.contains_key(&(T::PRIORITY, T::IDENTIFIER))
    }

    pub fn remove<T: GlobalStateController + State>(&mut self) -> Box<T> {
        self.try_remove().unwrap()
    }
    pub fn get<T: GlobalStateController + State>(&self) -> &T {
        self.try_get().unwrap()
    }
    pub fn get_mut<T: GlobalStateController + State>(&mut self) -> &mut T {
        self.try_get_mut().unwrap()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn GlobalStateController>> {
        self.states.values_mut()
    }
}
