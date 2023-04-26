use downcast_rs::{impl_downcast, Downcast};

use crate::Context;

pub trait SceneStateStaticAccess {
    fn get_update(&self) -> fn(&mut Context);
    fn get_end(&self) -> fn(&mut Context);
    fn get_after_update(&self) -> fn(&mut Context);
}

impl<T: SceneStateController> SceneStateStaticAccess for T {
    fn get_update(&self) -> fn(&mut Context) {
        T::update
    }

    fn get_end(&self) -> fn(&mut Context) {
        T::end
    }

    fn get_after_update(&self) -> fn(&mut Context) {
        T::after_update
    }
}

pub trait State {
    // Maybe add stuff here later ..
}

#[allow(unused_variables)]
pub trait SceneStateController: Downcast + SceneStateStaticAccess + State {
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

    fn after_update(ctx: &mut Context)
    where
        Self: Sized,
    {
    }
}

#[allow(unused_variables)]
pub trait GlobalStateController: Downcast + State {
    fn winit_event(&mut self, winit_event: &winit::event::Event<()>) {}
}

impl_downcast!(SceneStateController);
impl_downcast!(GlobalStateController);

impl State for () {}
impl SceneStateController for () {}
impl GlobalStateController for () {}
