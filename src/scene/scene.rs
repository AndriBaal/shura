#[cfg(feature = "physics")]
use crate::physics::World;

use crate::{
    ComponentManager, Context, GroupManager, ScreenConfig, Shura, StateManager, Vector,
    WorldCamera, WorldCameraScale,
};

/// Origin of a [Scene]
pub trait SceneCreator {
    fn new_id(&self) -> u32;
    fn create(self: Box<Self>, shura: &mut Shura) -> Scene;
}

/// Create a new [Scene] from scratch
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
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(mut self: Box<Self>, shura: &mut Shura) -> Scene {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let mut scene = Scene::new(window_size, self.id);
        scene.id = self.id;
        scene.started = true;
        let mut ctx = Context::new(shura, &mut scene);
        (self.init)(&mut ctx);
        return scene;
    }
}

/// Add a [Scene] that previously has been removed.
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
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(mut self: Box<Self>, shura: &mut Shura) -> Scene {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        self.scene.world_camera.resize(window_size);
        self.scene.id = self.id;
        self.scene.started = true;
        let mut ctx = Context::new(shura, &mut self.scene);
        (self.init)(&mut ctx);
        return self.scene;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
/// [states](StateManager) and [camera](WorldCamera) identified by an Id
pub struct Scene {
    pub(crate) id: u32,
    pub started: bool,
    pub render_components: bool,
    pub screen_config: ScreenConfig,
    pub world_camera: WorldCamera,
    pub components: ComponentManager,
    pub groups: GroupManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub states: StateManager,
    #[cfg(feature = "physics")]
    pub world: World,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(window_size: Vector<u32>, id: u32) -> Self {
        Self {
            id,
            world_camera: WorldCamera::new(
                Default::default(),
                WorldCameraScale::Min(Self::DEFAULT_VERTICAL_CAMERA_FOV),
                window_size,
            ),
            components: ComponentManager::new(),
            groups: GroupManager::new(),
            screen_config: ScreenConfig::new(),
            states: StateManager::default(),
            render_components: true,
            #[cfg(feature = "physics")]
            world: World::new(),
            started: true,
        }
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
