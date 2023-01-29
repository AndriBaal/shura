use crate::{DynamicScene, BaseScene};
use rustc_hash::FxHashMap;


/// Access to the scenes. (Removing)[crate::Context::remove_scene] and (creating)[crate::Context::create_scene]
/// scenes must be done from the (Context)[crate::Context].
pub struct SceneManager {
    scenes: FxHashMap<&'static str, DynamicScene>,
    future_active_scene: Option<&'static str>,
    curr_active_scene: &'static str,
}

impl SceneManager {
    pub(crate) fn new(active_scene: &'static str) -> Self {
        Self {
            scenes: FxHashMap::default(),
            future_active_scene: None,
            curr_active_scene: active_scene,
        }
    }

    pub fn does_scene_exist(&self, name: &'static str) -> bool {
        self.curr_active_scene == name || self.scenes.contains_key(&name)
    }

    pub(crate) fn add(&mut self, scene: DynamicScene) {
        let scene_name = scene.inner().name;
        if self.curr_active_scene == scene_name || self.scenes.contains_key(scene_name) {
            panic!("Scene {} does already exist!", scene_name);
        }
        self.scenes.insert(scene_name, scene);
    }

    /// Remove a scene by its name.
    ///
    /// # Panics
    /// Panics if the current scene is equal to the removed scene
    pub(crate) fn remove(&mut self, scene_name: &'static str) -> Option<DynamicScene> {
        if self.curr_active_scene == scene_name {
            panic!("Cannot remove the current active scene {}!", scene_name);
        }
        self.scenes.remove(scene_name)
    }

    // Getters
    #[inline]
    pub fn scenes(&self) -> Vec<&'static str> {
        self.scenes.keys().map(|k| *k).collect()
    }

    #[inline]
    pub const fn active_scene(&self) -> &'static str {
        self.curr_active_scene
    }

    #[inline]
    pub(crate) fn end_scenes(&mut self) -> FxHashMap<&'static str, DynamicScene> {
        std::mem::take(&mut self.scenes)
    }

    #[inline]
    pub(crate) fn resize(&mut self, main_scene: &mut DynamicScene) {
        main_scene.inner_mut().resized = true;
        for scene in self.scenes.values_mut() {
            scene.inner_mut().resized = true;
        }
    }

    // Setters

    #[inline]
    pub fn set_active_scene(&mut self, active_scene: &'static str) {
        if self.curr_active_scene == active_scene || self.scenes.contains_key(active_scene) {
            self.future_active_scene = Some(active_scene);
        } else {
            panic!("Cannot find the new active scene {}!", active_scene);
        }
    }

    pub(crate) fn apply_active_scene(&mut self) -> Option<DynamicScene> {
        let new_active = self.future_active_scene.take()?;
        let mut active = self
            .scenes
            .remove(new_active)
            .expect(format!("The main scene {} doesn't exist", new_active).as_str());
        self.curr_active_scene = new_active;
        active.inner_mut().switched = true;
        return Some(active);
    }
}
