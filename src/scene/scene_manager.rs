use crate::scene::Scene;
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

pub struct SceneManager {
    pub(crate) scenes: FxHashMap<u32, Rc<RefCell<Scene>>>,
    next_active_scene_id: u32,
    active_scene_id: u32,
    scene_switched: Option<u32>,
}

impl SceneManager {
    pub(crate) fn new(scene: Scene, active_scene_id: u32) -> Self {
        let mut scenes = Self {
            scenes: FxHashMap::default(),
            active_scene_id,
            next_active_scene_id: active_scene_id,
            scene_switched: None,
        };
        scenes.add(active_scene_id, scene);
        scenes
    }

    pub(crate) fn end_scenes(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (u32, Rc<RefCell<Scene>>)> {
        std::mem::take(&mut self.scenes).into_iter()
    }

    pub(crate) fn resize(&mut self) {
        for scene in self.scenes.values_mut() {
            let mut scene = scene.borrow_mut();
            scene.screen_config.changed = true;
        }
    }

    pub fn set_next_active_scene(&mut self, next_active_scene_id: u32) {
        assert!(
            self.scenes.contains_key(&next_active_scene_id),
            "Scene {next_active_scene_id} does not exist!"
        );
        self.next_active_scene_id = next_active_scene_id;
    }

    pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
        self.scenes.keys()
    }

    pub const fn active_scene_id(&self) -> u32 {
        self.active_scene_id
    }

    pub const fn next_active_scene_id(&self) -> u32 {
        self.next_active_scene_id
    }

    pub fn exists(&self, id: u32) -> bool {
        self.scenes.contains_key(&id)
    }

    pub fn switched(&self) -> Option<u32> {
        self.scene_switched
    }

    pub(crate) fn remove(&mut self, scene_id: u32) -> Option<Scene> {
        assert!(
            !scene_id != self.active_scene_id,
            "Cannot remove active scene {scene_id}!"
        );
        assert!(
            !scene_id != self.next_active_scene_id,
            "Cannot remove next active scene {scene_id}!"
        );
        self.scenes
            .remove(&scene_id)
            .map(|a| Rc::try_unwrap(a).ok().unwrap().into_inner())
    }

    pub(crate) fn get(&self, id: u32) -> Option<Rc<RefCell<Scene>>> {
        return self.scenes.get(&id).cloned();
    }

    pub fn add(&mut self, id: u32, scene: impl Into<Scene>) {
        let mut scene = scene.into();
        scene.systems.apply();
        assert!(!self.scenes.contains_key(&id), "Scene {id} already exists!");
        self.scenes.insert(id, Rc::new(RefCell::new(scene)));
    }

    pub(crate) fn get_active_scene(&mut self) -> Rc<RefCell<Scene>> {
        self.try_get_active_scene().unwrap_or_else(|| {
            panic!(
                "Cannot find the currently active scene {}!",
                self.active_scene_id
            )
        })
    }

    pub(crate) fn try_get_active_scene(&mut self) -> Option<Rc<RefCell<Scene>>> {
        if let Some(scene) = self.scenes.get(&self.next_active_scene_id) {
            self.scene_switched = if self.active_scene_id != self.next_active_scene_id {
                Some(self.active_scene_id)
            } else {
                None
            };
            self.active_scene_id = self.next_active_scene_id;
            Some(scene.clone())
        } else {
            None
        }
    }
}
