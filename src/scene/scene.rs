use crate::{
    entity::{
        Entities, EntityGroupManager, EntityIdentifier, EntityManager, EntityScope, EntityType,
        GroupedEntities, SingleEntity,
    },
    graphics::{
        CameraViewSelection, PerspectiveCamera3D, ScreenConfig, WorldCamera2D, WorldCamera3D,
        WorldCameraScaling,
    },
    math::Vector2,
    physics::World,
    system::{System, SystemManager},
    tasks::TaskManager,
};

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

    fn entity_grouped<E: EntityIdentifier>(self) -> Self
    where
        Self: Sized,
    {
        self.entity_custom(
            GroupedEntities::<Entities<E>>::default(),
            EntityScope::Scene,
        )
    }

    fn entity_grouped_single<E: EntityIdentifier>(self) -> Self
    where
        Self: Sized,
    {
        self.entity_custom(
            GroupedEntities::<SingleEntity<E>>::default(),
            EntityScope::Scene,
        )
    }

    fn entity_single<E: EntityIdentifier>(self) -> Self
    where
        Self: Sized,
    {
        self.entity_custom(SingleEntity::<E>::default(), EntityScope::Scene)
    }

    fn entity<E: EntityIdentifier>(self) -> Self
    where
        Self: Sized,
    {
        self.entity_custom(Entities::<E>::default(), EntityScope::Scene)
    }

    fn entity_single_global<E: EntityIdentifier>(self) -> Self
    where
        Self: Sized,
    {
        self.entity_custom(SingleEntity::<E>::default(), EntityScope::Global)
    }

    fn entity_global<E: EntityIdentifier>(self) -> Self
    where
        Self: Sized,
    {
        self.entity_custom(Entities::<E>::default(), EntityScope::Global)
    }

    fn entity_custom<ET: EntityType>(mut self, ty: ET, scope: EntityScope) -> Self
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
    #[cfg_attr(feature = "serde", serde(default))]
    pub(crate) started: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "EntityManager::new"))]
    pub(crate) entities: EntityManager,
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
            entities: EntityManager::new(),
            systems: SystemManager::new(),
            groups: EntityGroupManager::new(),
            screen_config: ScreenConfig::new(),
            render_entities: true,
            world: World::new(),
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
        }
    }
}

impl SceneCreator for Scene {
    fn scene(&mut self) -> &mut Scene {
        self
    }
}
