use core::panic;

use crate::{DynamicScene, SceneController};
use rustc_hash::FxHashMap;

/// Access to the scenes. (Removing)[crate::Context::remove_scene] and (creating)[crate::Context::create_scene]
/// scenes must be done from the (Context)[crate::Context].
pub struct SceneManager {
    scenes: FxHashMap<&'static str, Option<DynamicScene>>,
    active_scene: Option<&'static str>
}

impl SceneManager {
    pub(crate) fn new() -> Self {
        Self {
            scenes: FxHashMap::default(),
            active_scene: None,
        }
    }

    #[inline]
    pub(crate) fn init<S: SceneController>(&mut self, scene: S) {
        let scene_name = scene.base().name;
        self.active_scene = Some(scene_name);
        self.scenes.insert(scene_name, Some(Box::new(scene)));
    }

    pub fn does_scene_exist(&self, name: &'static str) -> bool {
        self.scenes.contains_key(&name)
    }

    pub(crate) fn add<S: SceneController>(&mut self, scene: S) {
        let scene_name = scene.base().name;
        if self.scenes.contains_key(scene_name) {
            panic!("Scene {} does already exist!", scene_name);
        }
        self.scenes.insert(scene_name, Some(Box::new(scene)));
    }

    /// Remove a scene by its name.
    ///
    /// # Panics
    /// Panics if the current scene is equal to the removed scene
    pub(crate) fn remove(&mut self, scene_name: &'static str) -> Option<DynamicScene> {
        if let Some(scene) = self.scenes.remove(scene_name) {
            if scene.is_none() {
                panic!("Cannot remove the currently active scene {}!", scene_name);
            }
            return scene;
        }
        return None;
    }

    // Getters
    #[inline]
    pub fn scenes(&self) -> Vec<&'static str> {
        self.scenes.keys().map(|k| *k).collect()
    }

    #[inline]
    pub const fn active_scene(&self) -> Option<&'static str> {
        self.active_scene
    }

    #[inline]
    pub(crate) fn end_scenes(&mut self) -> FxHashMap<&'static str, Option<DynamicScene>> {
        std::mem::take(&mut self.scenes)
    }

    #[inline]
    pub(crate) fn resize(&mut self) {
        for scene in self.scenes.values_mut() {
            scene.as_mut().unwrap().base_mut().resized = true;
        }
    }

    // Setters

    #[inline]
    pub fn set_active_scene(&mut self, active_scene: &'static str) {
        self.active_scene = Some(active_scene);
    }

    #[inline]
    pub(crate) fn borrow_active_scene(&mut self) -> DynamicScene {
        let active_scene =  self.active_scene.unwrap();
        if let Some(scene) = self.scenes.get_mut(active_scene) {
            return std::mem::replace(scene, None).unwrap();
        } else {
            panic!("Cannot find the new active scene {}!", active_scene);
        }
    }

    #[inline]
    pub(crate) fn return_active_scene(&mut self, scene: DynamicScene) {
        let scene_name =  scene.base().name();
        let _ = std::mem::replace(self.scenes.get_mut(scene_name).unwrap(), Some(scene));
    }
}
