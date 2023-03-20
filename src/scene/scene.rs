use crate::{ComponentManager, Context, ScreenConfig, ShuraFields, Vector, WorldCamera};

use super::state::SceneState;

pub trait SceneCreator {
    fn id(&self) -> u32;
    fn create(self, shura: ShuraFields) -> Scene;
}

pub struct NewScene<N: 'static + FnMut(&mut Context)> {
    pub id: u32,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context)> NewScene<N> {
    pub fn new(id: u32, init: N) -> NewScene<N> {
        Self { id, init }
    }
}

impl<N: 'static + FnMut(&mut Context)> SceneCreator for NewScene<N> {
    fn id(&self) -> u32 {
        self.id
    }

    fn create(mut self, shura: ShuraFields) -> Scene {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let window_ratio = window_size.x as f32 / window_size.y as f32;
        let mut scene = Scene::new(window_ratio, self.id);
        let mut ctx = Context::from_fields(shura, &mut scene);
        (self.init)(&mut ctx);
        scene.component_manager.update_sets(&scene.world_camera);
        return scene;
    }
}

pub struct RecycleScene<N: 'static + FnMut(&mut Context)> {
    pub id: u32,
    pub scene: Scene,
    pub init: N,
}

impl<N: 'static + FnMut(&mut Context)> RecycleScene<N> {
    pub fn new(id: u32, scene: Scene, init: N) -> RecycleScene<N> {
        Self { id, scene, init }
    }
}

impl<N: 'static + FnMut(&mut Context)> SceneCreator for RecycleScene<N> {
    fn id(&self) -> u32 {
        self.id
    }

    fn create(mut self, shura: ShuraFields) -> Scene {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let window_ratio = window_size.x as f32 / window_size.y as f32;
        self.scene.world_camera.resize(window_ratio);
        let mut ctx = Context::from_fields(shura, &mut self.scene);
        (self.init)(&mut ctx);
        self.scene.component_manager.update_sets(&self.scene.world_camera);
        return self.scene;
    }
}

fn default_state() -> Box<dyn SceneState> {
    return Box::new(());
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Scene {
    pub(crate) id: u32,
    pub(crate) resized: bool,
    pub(crate) switched: bool,
    pub screen_config: ScreenConfig,
    pub world_camera: WorldCamera,
    pub component_manager: ComponentManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "default_state"))]
    pub state: Box<dyn SceneState>,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(ratio: f32, id: u32) -> Self {
        Self {
            id: id,
            switched: true,
            resized: true,
            world_camera: WorldCamera::new(
                Default::default(),
                Self::DEFAULT_VERTICAL_CAMERA_FOV,
                ratio,
            ),
            component_manager: ComponentManager::new(),
            screen_config: ScreenConfig::new(),
            state: default_state(),
        }
    }

    pub fn resized(&self) -> bool {
        self.resized
    }

    pub fn switched(&self) -> bool {
        self.switched
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
