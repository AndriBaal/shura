use crate::{StateIdentifier, StateTypeId};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::BTreeMap;

#[allow(unused_variables)]
/// Defines a [State](crate::State) that can be appended to the core of shura. It can be used
/// to share data between all [Scenes](crate::Scene). For example global [Fonts](crate::text::FontBrush) or¨
/// other relevant data of the game.
pub trait GlobalStateController: Downcast {
    fn winit_event(&mut self, winit_event: &winit::event::Event<()>) {}
}
impl_downcast!(GlobalStateController);

#[derive(Default)]
/// Manager of [GlobalStates](crate::GlobalStateController)
pub struct GlobalStateManager {
    states: BTreeMap<(i16, StateTypeId), Box<dyn GlobalStateController>>,
}

impl GlobalStateManager {
    pub fn try_get<T: GlobalStateController + StateIdentifier>(&self) -> Option<&T> {
        self.states
            .get(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast_ref::<T>())
    }

    pub fn try_get_mut<T: GlobalStateController + StateIdentifier>(&mut self) -> Option<&mut T> {
        self.states
            .get_mut(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast_mut::<T>())
    }

    pub fn try_remove<T: GlobalStateController + StateIdentifier>(&mut self) -> Option<Box<T>> {
        self.states
            .remove(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast::<T>().ok())
    }

    pub fn insert<T: GlobalStateController + StateIdentifier>(&mut self, state: T) {
        self.states
            .insert((T::PRIORITY, T::IDENTIFIER), Box::new(state));
    }

    pub fn contains<T: GlobalStateController + StateIdentifier>(&self) -> bool {
        self.states.contains_key(&(T::PRIORITY, T::IDENTIFIER))
    }

    pub fn remove<T: GlobalStateController + StateIdentifier>(&mut self) -> Box<T> {
        self.try_remove().unwrap()
    }
    
    pub fn get<T: GlobalStateController + StateIdentifier>(&self) -> &T {
        self.try_get().unwrap()
    }
    
    pub fn get_mut<T: GlobalStateController + StateIdentifier>(&mut self) -> &mut T {
        self.try_get_mut().unwrap()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Box<dyn GlobalStateController>> {
        self.states.values_mut()
    }
}
