use crate::Context;
use downcast_rs::*;

/// Heap allocated scene that can be of any type. Can be downcasted with
/// [dowcast_ref](std::any::Any) or [dowcast_mut](std::any::Any).
pub type DynamicScene = Box<dyn SceneController>;

#[allow(unused_variables)]
/// Control the behaviour of a scene.
pub trait SceneController: Downcast {
    /// Update that gets called before updating the components.
    fn update(&mut self, ctx: &mut Context) {}
    /// Updates the scene after all components and after the physics step.
    fn after_update(&mut self, ctx: &mut Context) {}
    /// Gets called when the scene is removed or the game closes.
    fn end(&mut self, ctx: &mut Context) {}
}

impl_downcast!(SceneController);
impl<T: SceneController + ?Sized> SceneController for Box<T> {
    fn after_update(&mut self, ctx: &mut Context) {
        (**self).after_update(ctx)
    }
    fn end(&mut self, ctx: &mut Context) {
        (**self).end(ctx)
    }
    fn update(&mut self, ctx: &mut Context) {
        (**self).update(ctx)
    }
}
