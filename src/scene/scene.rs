use crate::{
    entity::{
        Entities, EntityIdentifier, EntityManager, EntityScope, EntityType, GroupManager,
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

pub trait SceneCreator {
    fn new_id(&self) -> u32;
    fn create(self: Box<Self>) -> Scene;
    fn scene(&mut self) -> &mut Scene;
    fn component<I: Instance>(mut self, name: &'static str, config: BufferConfig) -> Self
    where
        Self: Sized,
    {
        self.scene()
            .component_buffers
            .register_component::<I>(name, config);
        self
    }
    fn component2d(self, name: &'static str, config: BufferConfig) -> Self
    where
        Self: Sized,
    {
        self.component::<Instance2D>(name, config)
    }

    fn single_entity<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(SingleEntity::<E>::default(), scope)
    }

    fn entities<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(Entities::<E>::default(), scope)
    }

    fn grouped_entity<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(GroupedEntities::<Entities<E>>::default(), scope)
    }

    fn entity<ET: EntityType>(mut self, ty: ET, scope: EntityScope) -> Self
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

pub struct NewScene {
    id: u32,
    scene: Scene,
}

impl NewScene {
    pub fn new(id: u32) -> NewScene {
        Self {
            id,
            scene: Scene::new(),
        }
    }
}

impl SceneCreator for NewScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(self: Box<Self>) -> Scene {
        self.scene
    }

    fn scene(&mut self) -> &mut Scene {
        &mut self.scene
    }
}

// pub struct RecycleScene {
//     pub id: u32,
//     pub scene: Scene,
// }

// impl RecycleScene {
//     pub fn new(id: u32, scene: Scene) -> RecycleScene {
//         Self { id, scene }
//     }
// }

// impl SceneCreator for RecycleScene {
//     fn new_id(&self) -> u32 {
//         self.id
//     }

//     fn create(mut self: Box<Self>, app: &mut App) -> Scene {
//         let mint: mint::Vector2<u32> = app.window.inner_size().into();
//         let window_size: Vector2<u32> = mint.into();
//         self.scene.world_camera2d.resize(window_size);
//         // self.scene.component_buffers.init(self.component_buffers);
//         // self.scene.systems.init(&self.systems);
//         // self.scene.entities.init(&app.globals, self.entities);
//         self.scene
//     }
// }

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Scene {
    pub render_entities: bool,
    pub screen_config: ScreenConfig,
    pub world_camera2d: WorldCamera2D,
    pub world_camera3d: WorldCamera3D,
    pub groups: GroupManager,
    pub world: World,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "EntityManager::new"))]
    pub entities: EntityManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "SystemManager::new"))]
    pub systems: SystemManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "ComponentBufferManager::new"))]
    pub component_buffers: ComponentBufferManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "TaskManager::new"))]
    pub tasks: TaskManager,
}

impl Scene {
    pub(crate) fn new() -> Self {
        // let mint: mint::Vector2<u32> = window.inner_size().into();
        let window_size: Vector2<u32> = Vector2::new(800, 800);

        Self {
            entities: EntityManager::new(),
            systems: SystemManager::new(),
            groups: GroupManager::new(),
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
}
