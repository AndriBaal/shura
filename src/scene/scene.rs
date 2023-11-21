use std::cell::RefCell;

use rustc_hash::FxHashMap;

use crate::{
    App, BufferConfig, CameraViewSelection, ComponentBuffer, ComponentBufferImpl,
    ComponentBufferManager, Context, Entity, EntityConfig, EntityManager, EntityType,
    EntityTypeImplementation, GroupManager, Instance, PerspectiveCamera3D, ScreenConfig, System,
    SystemManager, TaskManager, Vector2, World, WorldCamera2D, WorldCamera3D, WorldCameraScaling,
    GLOBAL_GPU,
};

pub trait SceneCreator {
    fn new_id(&self) -> u32;
    fn create(self: Box<Self>, app: &mut App) -> Scene;
}

pub struct NewScene {
    pub id: u32,
    systems: Vec<System>,
    entities: Vec<Box<RefCell<dyn EntityTypeImplementation>>>,
    components: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl NewScene {
    pub fn new(id: u32) -> NewScene {
        Self {
            id,
            systems: Default::default(),
            entities: Default::default(),
            components: Default::default(),
        }
    }

    pub fn component<I: Instance>(mut self, name: &'static str, operation: BufferConfig) -> Self {
        if self.components.contains_key(name) {
            panic!("Component {} already defined!", name);
        }
        self.components.insert(
            name,
            Box::new(ComponentBuffer::<I>::new(GLOBAL_GPU.get().unwrap(), operation)),
        );
        self
    }

    pub fn entity<E: Entity>(mut self, config: EntityConfig) -> Self {
        self.entities
            .push(Box::new(RefCell::new(EntityType::<E>::new(config))));
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
        return Scene::new(self.id, app, self.systems, self.entities, self.components);
    }
}

pub struct RecycleScene {
    pub id: u32,
    pub scene: Scene,
    systems: Vec<System>,
    entities: Vec<Box<RefCell<dyn EntityTypeImplementation>>>,
    components: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl RecycleScene {
    pub fn new(id: u32, scene: Scene) -> RecycleScene {
        Self {
            id,
            scene,
            systems: Default::default(),
            entities: Default::default(),
            components: Default::default(),
        }
    }

    pub fn component<I: Instance>(mut self, name: &'static str, operation: BufferConfig) -> Self {
        if self.components.contains_key(name) {
            panic!("Component {} already defined!", name);
        }
        self.components.insert(
            name,
            Box::new(ComponentBuffer::<I>::new(GLOBAL_GPU.get().unwrap(), operation)),
        );
        self
    }

    pub fn entity<E: Entity>(mut self, config: EntityConfig) -> Self {
        self.entities
            .push(Box::new(RefCell::new(EntityType::<E>::new(config))));
        self
    }

    pub fn system(mut self, system: System) -> Self {
        self.systems.push(system);
        self
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
        self.scene.components.init(self.components);
        self.scene.systems.init(&self.systems);
        self.scene.entities.init(&app.globals, self.entities);
        return self.scene;
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct Scene {
    pub render_entities: bool,
    pub screen_config: ScreenConfig,
    pub world_camera2d: WorldCamera2D,
    pub world_camera3d: WorldCamera3D,
    pub groups: GroupManager,
    pub world: World,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "EntityManager::empty"))]
    pub entities: EntityManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "SystemManager::empty"))]
    pub systems: SystemManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "ComponentBufferManager::empty"))]
    pub components: ComponentBufferManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "TaskManager::new"))]
    pub tasks: TaskManager,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(
        id: u32,
        app: &mut App,
        systems: Vec<System>,
        entities: Vec<Box<RefCell<dyn EntityTypeImplementation>>>,
        components: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
    ) -> Self {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size: Vector2<u32> = mint.into();

        let mut scene = Self {
            world_camera2d: WorldCamera2D::new(
                window_size,
                Default::default(),
                WorldCameraScaling::Min(Self::DEFAULT_VERTICAL_CAMERA_FOV),
            ),
            entities: EntityManager::new(&app.globals, entities),
            systems: SystemManager::new(&systems),
            groups: GroupManager::new(),
            screen_config: ScreenConfig::new(),
            render_entities: true,
            world: World::new(),
            tasks: TaskManager::new(),
            world_camera3d: WorldCamera3D::new(
                window_size,
                CameraViewSelection::PerspectiveCamera3D(PerspectiveCamera3D::default()),
            ),
            components: ComponentBufferManager::new(components),
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
