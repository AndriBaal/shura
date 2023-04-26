use crate::{
    ComponentManager, Context, SceneStateController, ScreenConfig, ShuraFields, Vector,
    WorldCamera, WorldCameraScale,
};

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

fn default_state() -> Box<dyn SceneStateController> {
    return Box::new(());
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Scene {
    pub(crate) id: u32,
    pub(crate) resized: bool,
    pub(crate) switched: bool,
    pub(crate) started: bool,
    pub screen_config: ScreenConfig,
    pub world_camera: WorldCamera,
    pub component_manager: ComponentManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "default_state"))]
    pub state: Box<dyn SceneStateController>,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(window_size: Vector<u32>, id: u32) -> Self {
        Self {
            id: id,
            switched: true,
            resized: true,
            started: true,
            world_camera: WorldCamera::new(
                Default::default(),
                WorldCameraScale::Min(Self::DEFAULT_VERTICAL_CAMERA_FOV),
                window_size,
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

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
