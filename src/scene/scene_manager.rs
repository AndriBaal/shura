use crate::{Scene, DynamicScene};
use rustc_hash::FxHashMap;

pub(crate) type BoxedScene = (DynamicScene, Scene);

/// Access to the scenes. (Removing)[crate::Context::remove_scene] and (creating)[crate::Context::create_scene] 
/// scenes must be done from the (Context)[crate::Context].
pub struct SceneManager {
    scenes: FxHashMap<&'static str, BoxedScene>,
    future_active_scene: Option<&'static str>,
    curr_active_scene: &'static str,
}

impl SceneManager {
    pub(crate) fn new(active_scene: &'static str) -> Self {
        Self {
            scenes: FxHashMap::default(),
            future_active_scene: None,
            curr_active_scene: active_scene
        }
    }

    pub(crate) fn new_active_scene(&mut self) -> Option<&'static str> {
        return self.future_active_scene;
    }

    pub(crate) fn swap_active_scene(&mut self, old: BoxedScene, new: &'static str) -> BoxedScene {
        self.future_active_scene = None;
        let mut new_active = self
            .scenes
            .remove(new)
            .expect(format!("The main scene {} doesn't exist", new).as_str());
        new_active.1.switched = true;
        self.scenes.insert(old.1.name, old);
        return new_active;
    }

    pub fn does_scene_exist(&self, name: &'static str) -> bool {
        self.curr_active_scene == name || self.scenes.contains_key(&name)
    }

    pub(crate) fn add(&mut self, scene: BoxedScene) {
        let scene_name = scene.1.name;
        if self.curr_active_scene == scene_name || self.scenes.contains_key(scene_name) {
            panic!("Scene {} does already exist!", scene_name);
        }
        self.scenes.insert(scene_name, scene);
    }

    /// Remove a scene by its name.
    ///
    /// # Panics
    /// Panics if the current scene is equal to the removed scene
    pub(crate) fn remove(&mut self, scene_name: &'static str) -> Option <BoxedScene> {
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
    pub(crate) fn end_scenes(&mut self) -> FxHashMap<&'static str, BoxedScene> {
        std::mem::take(&mut self.scenes)
    }

    #[inline]
    pub(crate) fn resize(&mut self, main_scene: &mut BoxedScene) {
        main_scene.1.resized = true;
        for scene in self.scenes.values_mut() {
            scene.1.resized = true;
        }
    }

    // Setters

    #[inline]
    pub fn set_active_scene(&mut self, active_scene: &'static str) {
        if self.scenes.contains_key(active_scene) {
            self.future_active_scene = Some(active_scene);
        } else {
            panic!("Cannot find the new active scene {}!", active_scene);
        }
    }
}
