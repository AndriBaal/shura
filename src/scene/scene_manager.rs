use core::panic;
use rustc_hash::FxHashMap;

use crate::Scene;

/// Access to the scenes. (Removing)[crate::Context::remove_scene] and (creating)[crate::Context::create_scene]
/// scenes must be done from the (Context)[crate::Context].
pub struct SceneManager {
    scenes: FxHashMap<&'static str, Option<Scene>>,
    active_scene: &'static str,
}

impl SceneManager {
    pub(crate) fn new(active_scene: &'static str) -> Self {
        let mut scenes = FxHashMap::default();
        scenes.insert(active_scene, None);
        Self {
            scenes,
            active_scene,
        }
    }

    #[inline]
    pub(crate) fn init(&mut self, scene: Scene) {
        let scene_name = scene.name;
        self.scenes.insert(scene_name, Some(scene));
    }

    pub fn does_scene_exist(&self, name: &'static str) -> bool {
        self.scenes.contains_key(&name)
    }

    pub(crate) fn add(&mut self, scene: Scene) {
        let scene_name = scene.name;
        if self.scenes.contains_key(scene_name) {
            panic!("Scene {} does already exist!", scene_name);
        }
        self.scenes.insert(scene_name, Some(scene));
    }

    /// Remove a scene by its name.
    ///
    /// # Panics
    /// Panics if the current scene is equal to the removed scene
    pub(crate) fn remove(&mut self, scene_name: &'static str) -> Option<Scene> {
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
    pub const fn active_scene(&self) -> &'static str {
        self.active_scene
    }

    #[inline]
    pub(crate) fn end_scenes(&mut self) -> FxHashMap<&'static str, Option<Scene>> {
        std::mem::take(&mut self.scenes)
    }

    #[inline]
    pub(crate) fn resize(&mut self) {
        for scene in self.scenes.values_mut() {
            scene.as_mut().unwrap().resized = true;
        }
    }

    // Setters

    #[inline]
    pub fn set_active_scene(&mut self, active_scene: &'static str) {
        self.active_scene = active_scene;
    }

    #[inline]
    pub(crate) fn borrow_active_scene(&mut self) -> Scene {
        let active_scene = self.active_scene;
        if let Some(scene) = self.scenes.get_mut(active_scene) {
            return std::mem::replace(scene, None).unwrap();
        } else {
            panic!("Cannot find the new active scene {}!", active_scene);
        }
    }

    #[inline]
    pub(crate) fn return_active_scene(&mut self, scene: Scene) {
        let scene_name = scene.name();
        let _ = std::mem::replace(self.scenes.get_mut(scene_name).unwrap(), Some(scene));
    }
}
