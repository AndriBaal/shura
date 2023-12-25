use crate::scene::{Scene, SceneCreator};
use rustc_hash::FxHashMap;
use std::{cell::RefCell, rc::Rc};

pub struct SceneManager {
    pub(crate) scenes: FxHashMap<u32, Rc<RefCell<Scene>>>,
    pub(crate) remove: Vec<u32>,
    pub(crate) add: Vec<Box<dyn SceneCreator>>,
    active_scene_id: u32,
    last_active: Option<u32>,
    scene_switched: bool,
}

impl SceneManager {
    pub(crate) fn new(active_scene_id: u32, creator: impl SceneCreator + 'static) -> Self {
        Self {
            scenes: Default::default(),
            remove: Default::default(),
            add: vec![Box::new(creator)],
            active_scene_id,
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

    pub fn set_active_scene(&mut self, active_scene_id: u32) {
        self.active_scene_id = active_scene_id;
    }

    pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
        self.scenes.keys()
    }

    pub const fn active_scene_id(&self) -> u32 {
        self.active_scene_id
    }

    pub fn exists(&self, id: u32) -> bool {
        self.scenes.contains_key(&id)
    }

    pub fn switched(&self) -> bool {
        self.scene_switched
    }

    pub fn remove(&mut self, scene_id: u32) {
        self.remove.push(scene_id)
    }

    pub fn add(&mut self, scene: impl SceneCreator + 'static) {
        self.add.push(Box::new(scene))
    }

    // pub fn remove(&mut self, scene_id: u32) -> Option<Scene> {
    //     assert!(
    //         scene_id != self.active_scene_id,
    //         "Cannot remove active scene!"
    //     );
    //     self.scenes.remove(&scene_id).and_then(|s| {
    //         Some(
    //             Rc::try_unwrap(s)
    //                 .ok()
    //                 .expect("Scene already in use!")
    //                 .into_inner(),
    //         )
    //     })
    // }

    pub(crate) fn get_active_scene(&mut self) -> Rc<RefCell<Scene>> {
        self.try_get_active_scene().unwrap_or_else(|| {
            panic!(
                "Cannot find the currently active scene {}!",
                self.active_scene_id
            )
        })
    }

    pub(crate) fn try_get_active_scene(&mut self) -> Option<Rc<RefCell<Scene>>> {
        if let Some(scene) = self.scenes.get(&self.active_scene_id) {
            if let Some(last) = self.last_active {
                self.scene_switched = last != self.active_scene_id;
            } else {
                self.scene_switched = true;
            }
            self.last_active = Some(self.active_scene_id);
            Some(scene.clone())
        } else {
            None
        }
    }
}
