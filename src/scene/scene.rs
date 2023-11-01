use std::cell::RefCell;

use crate::{
    App, Component, ComponentConfig, ComponentManager, ComponentType, ComponentTypeImplementation,
    Context, GroupManager, ScreenConfig, System, SystemManager, Vector2, World, WorldCamera2D,
    WorldCameraScaling,
};

/// Origin of a [Scene]
pub trait SceneCreator {
    fn new_id(&self) -> u32;
    fn create(self: Box<Self>, app: &mut App) -> Scene;
}

/// Create a new [Scene] from scratch
pub struct NewScene {
    pub id: u32,
    systems: Vec<System>,
    components: Vec<Box<RefCell<dyn ComponentTypeImplementation>>>,
}

impl NewScene {
    pub fn new(id: u32) -> NewScene {
        Self {
            id,
            systems: Default::default(),
            components: Default::default(),
        }
    }

    pub fn component<C: Component>(mut self, config: ComponentConfig) -> Self {
        self.components
            .push(Box::new(RefCell::new(ComponentType::<C>::new(config))));
        self
    }

    pub fn system(mut self, system: System) -> Self {
        self.systems.push(system);
        self
    }
}

impl SceneCreator for NewScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(self: Box<Self>, app: &mut App) -> Scene {
        return Scene::new(self.id, app, self.systems, self.components);
    }
}

/// Add a [Scene] that previously has been removed.
pub struct RecycleScene {
    pub id: u32,
    pub scene: Scene,
}

impl RecycleScene {
    pub fn new(id: u32, scene: Scene) -> RecycleScene {
        Self { id, scene }
    }
}

impl SceneCreator for RecycleScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(mut self: Box<Self>, app: &mut App) -> Scene {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size: Vector2<u32> = mint.into();
        self.scene.world_camera2d.resize(window_size);
        return self.scene;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
/// [states](StateManager) and [camera](WorldCamera) identified by an Id
pub struct Scene {
    pub render_components: bool,
    pub screen_config: ScreenConfig,
    pub world_camera2d: WorldCamera2D,
    pub groups: GroupManager,
    pub world: World,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "ComponentManager::empty"))]
    pub components: ComponentManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "SystemManager::empty"))]
    pub systems: SystemManager,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(
        id: u32,
        app: &mut App,
        systems: Vec<System>,
        components: Vec<Box<RefCell<dyn ComponentTypeImplementation>>>,
    ) -> Self {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size: Vector2<u32> = mint.into();

        let mut scene = Self {
            world_camera2d: WorldCamera2D::new(
                Default::default(),
                WorldCameraScaling::Min(Self::DEFAULT_VERTICAL_CAMERA_FOV),
                window_size,
            ),
            components: ComponentManager::new(&app.globals, components),
            systems: SystemManager::new(&systems),
            groups: GroupManager::new(),
            screen_config: ScreenConfig::new(),
            render_components: true,
            world: World::new(),
        };

        let (_, mut ctx) = Context::new(&id, app, &mut scene);
        for system in &systems {
            match system {
                System::Setup(setup) => {
                    (setup)(&mut ctx);
                }
                _ => (),
            }
        }

        return scene;
    }
}
