use downcast_rs::{impl_downcast, Downcast};

use crate::Context;

#[allow(unused_variables)]
pub trait SceneStateController: Downcast {
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

#[allow(unused_variables)]
pub trait GlobalStateController: Downcast {
    fn winit_event(&mut self, winit_event: &winit::event::Event<()>) {}
}

impl_downcast!(SceneStateController);
impl_downcast!(GlobalStateController);

impl SceneStateController for () {}
impl GlobalStateController for () {}

pub struct SceneState {
    inner: Box<dyn SceneStateController>,
    pub(crate) update: fn(ctx: &mut Context),
    pub(crate) end: fn(ctx: &mut Context),
}

impl SceneState {
    pub fn new<S: SceneStateController>(state: S) -> Self {
        Self {
            inner: Box::new(state),
            update: S::update,
            end: S::end,
        }
    }

    pub fn set<S: SceneStateController>(&mut self, state: S) {
        self.inner = Box::new(state);
        self.update = S::update;
        self.end = S::end;
    }

    pub fn get<S: SceneStateController>(&self) -> Option<&S> {
        self.inner.downcast_ref::<S>()
    }

    pub fn get_mut<S: SceneStateController>(&mut self) -> Option<&mut S> {
        self.inner.downcast_mut::<S>()
    }

    pub fn take<S: SceneStateController>(&mut self) -> Option<S> {
        let state = std::mem::replace(self, Self::new(()));
        return state.inner.downcast::<S>().ok().and_then(|s| Some(*s));
    }
}

pub struct GlobalState {
    pub(crate) inner: Box<dyn GlobalStateController>,
}

impl GlobalState {
    pub fn new<G: GlobalStateController>(state: G) -> Self {
        Self {
            inner: Box::new(state),
        }
    }

    pub fn set<G: GlobalStateController>(&mut self, state: G) {
        self.inner = Box::new(state);
    }

    pub fn get<G: GlobalStateController>(&self) -> Option<&G> {
        self.inner.downcast_ref::<G>()
    }

    pub fn get_mut<G: GlobalStateController>(&mut self) -> Option<&mut G> {
        self.inner.downcast_mut::<G>()
    }

    pub fn take<G: GlobalStateController>(&mut self) -> Option<G> {
        let state = std::mem::replace(self, Self::new(()));
        return state.inner.downcast::<G>().ok().and_then(|s| Some(*s));
    }
}
