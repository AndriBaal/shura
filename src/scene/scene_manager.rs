use core::panic;
use rustc_hash::FxHashMap;

use crate::Scene;

/// Access to the scenes. [Removing](crate::Context::remove_scene) and [creating](crate::Context::add_scene)
/// scenes must be done from the [Context](crate::Context).
pub struct SceneManager {
    scenes: FxHashMap<u32, Option<Scene>>,
    active_scene: u32,
}

impl SceneManager {
    pub(crate) fn new(active_scene: u32) -> Self {
        let mut scenes = FxHashMap::default();
        scenes.insert(active_scene, None);
        Self {
            scenes,
            active_scene,
        }
    }

    pub(crate) fn init(&mut self, scene: Scene) {
        let scene_id = scene.id;
        self.scenes.insert(scene_id, Some(scene));
    }

    pub fn does_scene_exist(&self, id: u32) -> bool {
        self.scenes.contains_key(&id)
    }

    pub(crate) fn add(&mut self, scene: Scene) {
        let scene_id = scene.id;
        if self.scenes.contains_key(&scene_id) {
            panic!("Scene {} does already exist!", scene_id);
        }
        self.scenes.insert(scene_id, Some(scene));
    }

    /// Remove a scene by its id.
    ///
    /// # Panics
    /// Panics if the current scene is equal to the removed scene
    pub(crate) fn remove(&mut self, scene_id: u32) -> Option<Scene> {
        if let Some(scene) = self.scenes.remove(&scene_id) {
            if scene.is_none() {
                panic!("Cannot remove the currently active scene {}!", scene_id);
            }
            return scene;
        }
        return None;
    }

    pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
        self.scenes.keys().into_iter()
    }

    pub const fn active_scene(&self) -> u32 {
        self.active_scene
    }

    pub(crate) fn end_scenes(&mut self) -> impl Iterator<Item = (u32, Option<Scene>)> {
        std::mem::take(&mut self.scenes).into_iter()
    }

    pub(crate) fn resize(&mut self) {
        for scene in self.scenes.values_mut() {
            scene.as_mut().unwrap().resized = true;
        }
    }

    pub fn set_active_scene(&mut self, active_scene: u32) {
        self.active_scene = active_scene;
    }

    pub(crate) fn borrow_active_scene(&mut self) -> Scene {
        let active_scene = self.active_scene;
        if let Some(scene) = self.scenes.get_mut(&active_scene) {
            return std::mem::replace(scene, None).unwrap();
        } else {
            panic!("Cannot find the currently active scene {}!", active_scene);
        }
    }

    pub(crate) fn return_active_scene(&mut self, scene: Scene) {
        let scene_id = scene.id();
        let _ = std::mem::replace(self.scenes.get_mut(&scene_id).unwrap(), Some(scene));
    }
}
