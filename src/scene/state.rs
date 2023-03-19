use downcast_rs::{impl_downcast, Downcast};

use crate::Context;

pub trait SceneStateStaticAccess {
    fn get_update(&self) -> fn(&mut Context);
    fn get_end(&self) -> fn(&mut Context);
}

impl<T: SceneState> SceneStateStaticAccess for T {
    fn get_update(&self) -> fn(&mut Context) {
        T::update
    }

    fn get_end(&self) -> fn(&mut Context) {
        T::end
    }
}

#[allow(unused_variables)]
pub trait SceneState: Downcast + SceneStateStaticAccess {
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
pub trait GlobalState: Downcast {
    fn winit_event(&mut self, winit_event: &winit::event::Event<()>) {}
}

impl_downcast!(SceneState);
impl_downcast!(GlobalState);

impl SceneState for () {}
impl GlobalState for () {}

impl<T: SceneState + ?Sized> SceneState for Box<T> {}
impl<T: SceneState + ?Sized> GlobalState for Box<T> {}
