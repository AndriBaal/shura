#[cfg(feature = "physics")]
use crate::physics::World;

use crate::{
    ComponentManager, Context, SceneStateManager, ScreenConfig, ShuraFields, Vector, WorldCamera,
    WorldCameraScale,
};

/// Origin of a [Scene]
pub trait SceneCreator {
    fn id(&self) -> u32;
    fn create(self, shura: ShuraFields) -> Scene;
    fn scene(self, shura: ShuraFields) -> Scene
    where
        Self: Sized,
    {
        let id = self.id();
        let mut scene = self.create(shura);
        scene.id = id;
        scene.started = true;
        scene.resized = true;
        scene.switched = true;
        return scene;
    }
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
    fn id(&self) -> u32 {
        self.id
    }

    fn create(mut self, shura: ShuraFields) -> Scene {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();
        let mut scene = Scene::new(window_size, self.id);
        let mut ctx = Context::from_fields(shura, &mut scene);
        (self.init)(&mut ctx);
        return scene;
    }
}

/// Add a [Scene] that previously has been removed by calling [remove_scene](crate::Context::remove_scene)
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
        self.scene.world_camera.resize(window_size);
        let mut ctx = Context::from_fields(shura, &mut self.scene);
        (self.init)(&mut ctx);
        return self.scene;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
/// Scene owning its own [Components](ComponentManager), [Configurations](ScreenConfig), callbacks(resized, switched, started),
/// [states](SceneStateManager) and [camera](WorldCamera) identified by an Id
pub struct Scene {
    pub(crate) id: u32,
    pub(crate) resized: bool,
    pub(crate) switched: bool,
    pub(crate) started: bool,
    pub render_components: bool,
    pub screen_config: ScreenConfig,
    pub world_camera: WorldCamera,
    pub components: ComponentManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub states: SceneStateManager,
    #[cfg(feature = "physics")]
    pub world: World,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(window_size: Vector<u32>, id: u32) -> Self {
        Self {
            render_components: true,
            id: id,
            switched: true,
            resized: true,
            started: true,
            world_camera: WorldCamera::new(
                Default::default(),
                WorldCameraScale::Min(Self::DEFAULT_VERTICAL_CAMERA_FOV),
                window_size,
            ),
            components: ComponentManager::new(),
            screen_config: ScreenConfig::new(),
            states: SceneStateManager::default(),
            #[cfg(feature = "physics")]
            world: World::new(),
        }
    }

    pub fn resized(&self) -> bool {
        self.resized
    }

    pub fn switched(&self) -> bool {
        self.switched
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
