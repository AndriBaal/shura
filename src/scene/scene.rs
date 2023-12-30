use crate::{
    entity::{
        Entities, EntityGroupManager, EntityIdentifier, EntityManager, EntityScope, EntityType,
        GroupedEntities, SingleEntity,
    },
    graphics::{
        BufferConfig, CameraViewSelection, ComponentBufferManager, Instance, Instance2D,
        PerspectiveCamera3D, ScreenConfig, WorldCamera2D, WorldCamera3D, WorldCameraScaling,
    },
    math::Vector2,
    physics::World,
    system::{System, SystemManager},
    tasks::TaskManager,
};

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
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
    #[cfg_attr(feature = "serde", serde(default = "ComponentBufferManager::new"))]
    pub(crate) component_buffers: ComponentBufferManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "TaskManager::new"))]
    pub(crate) tasks: TaskManager,
}

impl Scene {
    pub fn new() -> Self {
        // let mint: mint::Vector2<u32> = window.inner_size().into();
        let window_size: Vector2<u32> = Vector2::new(800, 800);

        Self {
            entities: EntityManager::new(),
            systems: SystemManager::new(),
            groups: EntityGroupManager::new(),
            screen_config: ScreenConfig::new(),
            render_entities: true,
            world: World::new(),
            tasks: TaskManager::new(),
            component_buffers: ComponentBufferManager::new(),
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

    pub fn component<I: Instance>(mut self, name: &'static str, config: BufferConfig) -> Self
    where
        Self: Sized,
    {
        self.component_buffers.register_component::<I>(name, config);
        self
    }
    pub fn component2d(self, name: &'static str, config: BufferConfig) -> Self
    where
        Self: Sized,
    {
        self.component::<Instance2D>(name, config)
    }

    pub fn single_entity<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(SingleEntity::<E>::default(), scope)
    }

    pub fn entities<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(Entities::<E>::default(), scope)
    }

    pub fn grouped_entity<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(GroupedEntities::<Entities<E>>::default(), scope)
    }

    pub fn entity<ET: EntityType>(mut self, ty: ET, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entities.register_entity::<ET>(scope, ty);
        self
    }

    pub fn system(mut self, system: System) -> Self
    where
        Self: Sized,
    {
        self.systems.register_system(system);
        self
    }
}
