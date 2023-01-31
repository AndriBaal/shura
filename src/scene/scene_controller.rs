use crate::{BaseScene, Shura};
use downcast_rs::*;

/// Heap allocated scene that can be of any type. Can be downcasted with
/// [dowcast_ref](std::any::Any) or [dowcast_mut](std::any::Any).
pub type DynamicScene = Box<dyn SceneController>;

#[allow(unused_variables)]
/// Control the behaviour of a scene.
pub trait SceneController: Downcast + SceneDerive {
    /// Update that gets called before updating the components.
    fn update(&mut self, shura: &mut Shura) {}
    /// Updates the scene after all components and after the physics step.
    fn after_update(&mut self, shura: &mut Shura) {}
    /// Gets called when the scene is removed or the game closes.
    fn end(&mut self, shura: &mut Shura) {}
}

impl_downcast!(SceneController);
impl<T: SceneController + ?Sized> SceneController for Box<T> {
    fn after_update(&mut self, shura: &mut Shura) {
        (**self).after_update(shura)
    }
    fn end(&mut self, shura: &mut Shura) {
        (**self).end(shura)
    }
    fn update(&mut self, shura: &mut Shura) {
        (**self).update(shura)
    }
}

pub trait SceneDerive {
    fn base(&self) -> &BaseScene;
    fn base_mut(&mut self) -> &mut BaseScene;
}

impl<C: SceneController + ?Sized> SceneDerive for Box<C> {
    fn base(&self) -> &BaseScene {
        (**self).base()
    }

    fn base_mut(&mut self) -> &mut BaseScene {
        (**self).base_mut()
    }
}

impl SceneDerive for BaseScene {
    fn base(&self) -> &BaseScene {
        self
    }
    fn base_mut(&mut self) -> &mut BaseScene {
        self
    }
}
