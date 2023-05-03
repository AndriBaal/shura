use crate::{Context, State, StateTypeId};
use downcast_rs::{impl_downcast, Downcast};
use std::collections::BTreeMap;

pub trait SceneStateStaticAccess {
    fn get_update(&self) -> fn(&mut Context);
    fn get_end(&self) -> fn(&mut Context);
}

impl<T: SceneStateController> SceneStateStaticAccess for T {
    fn get_update(&self) -> fn(&mut Context) {
        T::update
    }

    fn get_end(&self) -> fn(&mut Context) {
        T::end
    }
}

#[allow(unused_variables)]
pub trait SceneStateController: Downcast + SceneStateStaticAccess {
    fn update(ctx: &mut Context)
    where
        Self: Sized,
    {
    }
    fn end(ctx: &mut Context)
    where
        Self: Sized,
    {
    }
}
impl_downcast!(SceneStateController);

#[derive(Default)]
pub struct SceneStateManager {
    states: BTreeMap<(i16, StateTypeId), Box<dyn SceneStateController>>,
}

impl SceneStateManager {
    pub fn try_get<T: SceneStateController + State>(&self) -> Option<&T> {
        self.states
            .get(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast_ref::<T>())
    }

    pub fn try_get_mut<T: SceneStateController + State>(&mut self) -> Option<&mut T> {
        self.states
            .get_mut(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast_mut::<T>())
    }

    pub fn try_remove<T: SceneStateController + State>(&mut self) -> Option<Box<T>> {
        self.states
            .remove(&(T::PRIORITY, T::IDENTIFIER))
            .and_then(|s| s.downcast::<T>().ok())
    }

    pub fn insert<T: SceneStateController + State>(&mut self, state: T) {
        self.states
            .insert((T::PRIORITY, T::IDENTIFIER), Box::new(state));
    }

    pub fn contains<T: SceneStateController + State>(&self) -> bool {
        self.states.contains_key(&(T::PRIORITY, T::IDENTIFIER))
    }

    pub fn remove<T: SceneStateController + State>(&mut self) -> Box<T> {
        self.try_remove().unwrap()
    }
    pub fn get<T: SceneStateController + State>(&self) -> &T {
        self.try_get().unwrap()
    }
    pub fn get_mut<T: SceneStateController + State>(&mut self) -> &mut T {
        self.try_get_mut().unwrap()
    }

    pub(crate) fn updates(&self, last_prio: i16, this_prio: i16) -> Vec<fn(&mut Context)> {
        use std::ops::Bound::{Excluded, Included};
        self.states
            .range((
                if last_prio == i16::MIN {
                    Included((last_prio, StateTypeId::new(0)))
                } else {
                    Excluded((last_prio, StateTypeId::new(0)))
                },
                Included((this_prio, StateTypeId::new(0))),
            ))
            .map(|c| c.1.get_update())
            .collect()
    }

    pub(crate) fn ends(&self) -> Vec<fn(&mut Context)> {
        self.states.iter().map(|c| c.1.get_end()).collect()
    }
}
