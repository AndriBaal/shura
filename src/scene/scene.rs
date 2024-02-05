use crate::{
    entity::{
        Entities, EntityGroupManager, EntityIdentifier, EntityManager, EntityScope, EntityStorage,
        EntityType, GroupedEntities, SingleEntity,
    },
    graphics::{
        CameraViewSelection, Instance, Instance2D, Instance3D, PerspectiveCamera3D,
        RenderGroupConfig, RenderGroupManager, ScreenConfig, WorldCamera2D, WorldCamera3D,
        WorldCameraScaling,
    },
    math::Vector2,
    physics::World,
    system::{System, SystemManager},
    tasks::TaskManager,
};

pub trait SceneCreator {
    fn scene(&mut self) -> &mut Scene;

    fn render_group<I: Instance>(mut self, name: &'static str, config: RenderGroupConfig) -> Self
    where
        Self: Sized,
    {
        self.scene()
            .render_groups
            .register_component::<I>(name, config);
        self
    }
    fn entity<E: EntityIdentifier>(self, storage: EntityStorage, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        match storage {
            EntityStorage::Single => self.custom_entity(SingleEntity::<E>::default(), scope),
            EntityStorage::Multiple => self.custom_entity(Entities::<E>::default(), scope),
            EntityStorage::Groups => {
                self.custom_entity(GroupedEntities::<Entities<E>>::default(), scope)
            }
        }
    }
    fn custom_entity<ET: EntityType>(mut self, ty: ET, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.scene().entities.register_entity::<ET>(scope, ty);
        self
    }

    fn system(mut self, system: System) -> Self
    where
        Self: Sized,
    {
        self.scene().systems.register_system(system);
        self
    }

    fn render_group2d(self, name: &'static str, config: RenderGroupConfig) -> Self
    where
        Self: Sized,
    {
        self.render_group::<Instance2D>(name, config)
    }

    fn render_group3d(self, name: &'static str, config: RenderGroupConfig) -> Self
    where
        Self: Sized,
    {
        self.render_group::<Instance3D>(name, config)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
#[non_exhaustive]
pub struct Scene {
    pub(crate) render_entities: bool,
    pub(crate) screen_config: ScreenConfig,
    pub(crate) world_camera2d: WorldCamera2D,
    pub(crate) world_camera3d: WorldCamera3D,
    pub(crate) groups: EntityGroupManager,
    pub(crate) world: World,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "EntityManager::new"))]
    pub(crate) entities: EntityManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "SystemManager::new"))]
    pub(crate) systems: SystemManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "RenderGroupManager::new"))]
    pub(crate) render_groups: RenderGroupManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "TaskManager::new"))]
    pub(crate) tasks: TaskManager,
}

impl Scene {
    pub fn new() -> Self {
        let window_size: Vector2<u32> = Vector2::new(800, 800);

        Self {
            entities: EntityManager::new(),
            systems: SystemManager::new(),
            groups: EntityGroupManager::new(),
            screen_config: ScreenConfig::new(),
            render_entities: true,
            world: World::new(),
            tasks: TaskManager::new(),
            render_groups: RenderGroupManager::new(),
            world_camera3d: WorldCamera3D::new(
                window_size,
                CameraViewSelection::PerspectiveCamera3D(PerspectiveCamera3D::default()),
            ),

            world_camera2d: WorldCamera2D::new(
                window_size,
                Default::default(),
                WorldCameraScaling::Min(WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV),
            ),
        }
    }
}

impl SceneCreator for Scene {
    fn scene(&mut self) -> &mut Scene {
        self
    }
}
