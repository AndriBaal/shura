use crate::{Scene, SceneCreator};
use core::panic;
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

/// Access to the scenes.
pub struct SceneManager {
    pub(crate) scenes: FxHashMap<u32, Rc<RefCell<Scene>>>,
    pub(crate) remove: Vec<u32>,
    pub(crate) add: Vec<Box<dyn SceneCreator>>,
    active_scene: u32,
    last_active: Option<u32>,
    scene_switched: bool,
}

impl SceneManager {
    pub(crate) fn new(active_scene: u32, creator: impl SceneCreator + 'static) -> Self {
        Self {
            active_scene,
            remove: Default::default(),
            scenes: Default::default(),
            add: vec![Box::new(creator)],
            last_active: None,
            scene_switched: false,
        }
    }

    pub(crate) fn end_scenes(&mut self) -> impl Iterator<Item = (u32, Rc<RefCell<Scene>>)> {
        std::mem::take(&mut self.scenes).into_iter()
    }

    pub(crate) fn resize(&mut self) {
        for scene in self.scenes.values_mut() {
            let mut scene = scene.borrow_mut();
            scene.screen_config.changed = true;
        }
    }

    pub fn set_active_scene(&mut self, active_scene: u32) {
        self.active_scene = active_scene;
    }

    pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
        self.scenes.keys().into_iter()
    }

    pub const fn active_scene(&self) -> u32 {
        self.active_scene
    }

    pub fn exists(&self, id: u32) -> bool {
        self.scenes.contains_key(&id)
    }

    pub fn add(&mut self, scene: impl SceneCreator + 'static) {
        self.add.push(Box::new(scene))
    }

    pub fn switched(&self) -> bool {
        self.scene_switched
    }

    /// Remove a scene by its id.
    pub fn remove(&mut self, scene_id: u32) {
        self.remove.push(scene_id)
    }

    pub(crate) fn get_active_scene(&mut self) -> Rc<RefCell<Scene>> {
        if let Some(scene) = self.scenes.get(&self.active_scene) {
            if let Some(last) = self.last_active {
                self.scene_switched = last != self.active_scene;
            } else {
                self.scene_switched = true;
            }
            self.last_active = Some(self.active_scene);
            return scene.clone();
        } else {
            panic!(
                "Cannot find the currently active scene {}!",
                self.active_scene
            );
        }
    }
}
