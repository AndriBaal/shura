use crate::{
    ecs::{System, SystemManager, World},
    graphics::{
        CameraViewSelection, PerspectiveCamera3D, ScreenConfig, WorldCamera2D, WorldCamera3D,
        WorldCameraScaling,
    },
    math::Vector2,
    tasks::TaskManager,
};

#[cfg(feature="physics")]
use crate::physics::Physics;

pub trait Plugin {
    fn init<S: SceneCreator>(&mut self, scene: S) -> S;
}

pub trait SceneCreator {
    fn scene(&mut self) -> &mut Scene;
    fn plugin(self, mut plugin: impl Plugin) -> Self
    where
        Self: Sized,
    {
        plugin.init(self)
    }
    fn system(mut self, system: System) -> Self
    where
        Self: Sized,
    {
        self.scene().systems.register_system(system);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[non_exhaustive]
pub struct Scene {
    pub(crate) render_entities: bool,
    pub(crate) screen_config: ScreenConfig,
    pub(crate) world_camera2d: WorldCamera2D,
    pub(crate) world_camera3d: WorldCamera3D,
    pub(crate) world: World,
    #[cfg(feature="physics")]
    pub(crate) physics: Physics,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) started: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "SystemManager::new"))]
    pub(crate) systems: SystemManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "TaskManager::new"))]
    pub(crate) tasks: TaskManager,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        let window_size: Vector2<u32> = Vector2::new(800, 800);

        Self {
            // entities: EntityManager::new(),
            // groups: EntityGroupManager::new(),
            systems: SystemManager::new(),
            screen_config: ScreenConfig::new(),
            render_entities: true,
            #[cfg(feature="physics")]
            physics: Physics::new(),
            tasks: TaskManager::new(),
            world_camera3d: WorldCamera3D::new(
                window_size,
                CameraViewSelection::PerspectiveCamera3D(PerspectiveCamera3D::default()),
            ),

            world_camera2d: WorldCamera2D::new(
                window_size,
                Default::default(),
                WorldCameraScaling::Min(WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV),
            ),
            started: false,
            world: World::new(),
        }
    }
}

impl SceneCreator for Scene {
    fn scene(&mut self) -> &mut Scene {
        self
    }
}
