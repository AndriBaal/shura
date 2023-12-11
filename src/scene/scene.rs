use rustc_hash::FxHashMap;
use std::{any::TypeId, cell::RefCell};

use crate::{
    app::{GLOBAL_GPU, App},
    entity::{Entities, EntityIdentifier, EntityScope, GroupedEntities, SingleEntity, EntityType, EntityManager, GroupManager},
    graphics::{
        BufferConfig, CameraViewSelection, ComponentBuffer, ComponentBufferImpl,
        ComponentBufferManager, Instance,
        PerspectiveCamera3D, ScreenConfig,
        WorldCamera2D, WorldCamera3D, WorldCameraScaling,
    },
    system::{System, SystemManager},
    context::Context,
    math::Vector2,
    tasks::TaskManager,
    physics::World
};

pub trait SceneCreator {
    fn new_id(&self) -> u32;
    fn create(self: Box<Self>, app: &mut App) -> Scene;
    fn systems(&mut self) -> &mut Vec<System>;
    fn entities(&mut self) -> &mut Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>;
    fn components(&mut self) -> &mut FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>;
    fn component<I: Instance>(mut self, name: &'static str, buffer: BufferConfig) -> Self
    where
        Self: Sized,
    {
        if self.components().contains_key(name) {
            panic!("Component {} already defined!", name);
        }
        self.components().insert(
            name,
            Box::new(ComponentBuffer::<I>::new(GLOBAL_GPU.get().unwrap(), buffer)),
        );
        self
    }

    fn entity_single<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(SingleEntity::<E>::default(), scope)
    }

    fn entity_multiple<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(Entities::<E>::default(), scope)
    }

    fn entity_grouped<E: EntityIdentifier>(self, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        self.entity(GroupedEntities::<Entities<E>>::default(), scope)
    }

    fn entity<T: EntityType>(mut self, ty: T, scope: EntityScope) -> Self
    where
        Self: Sized,
    {
        if TypeId::of::<T>() == TypeId::of::<GroupedEntities<T>>() && scope == EntityScope::Global {
            panic!(
                "Global component can not be stored in groups because groups are scene specific!"
            );
        }

        self.entities().push((scope, Box::new(RefCell::new(ty))));
        self
    }

    fn system(mut self, system: System) -> Self
    where
        Self: Sized,
    {
        self.systems().push(system);
        self
    }
}

pub struct NewScene {
    pub id: u32,
    systems: Vec<System>,
    entities: Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>,
    component_buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl NewScene {
    pub fn new(id: u32) -> NewScene {
        Self {
            id,
            systems: Default::default(),
            entities: Default::default(),
            component_buffers: Default::default(),
        }
    }
}

#[allow(private_interfaces)]
impl SceneCreator for NewScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(self: Box<Self>, app: &mut App) -> Scene {
        Scene::new(
            self.id,
            app,
            self.systems,
            self.entities,
            self.component_buffers,
        )
    }

    fn systems(&mut self) -> &mut Vec<System> {
        &mut self.systems
    }

    fn entities(&mut self) -> &mut Vec<(EntityScope, Box<RefCell<dyn EntityType>>)> {
        &mut self.entities
    }

    fn components(&mut self) -> &mut FxHashMap<&'static str, Box<dyn ComponentBufferImpl>> {
        &mut self.component_buffers
    }
}

pub struct RecycleScene {
    pub id: u32,
    pub scene: Scene,
    systems: Vec<System>,
    entities: Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>,
    component_buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
}

impl RecycleScene {
    pub fn new(id: u32, scene: Scene) -> RecycleScene {
        Self {
            id,
            scene,
            systems: Default::default(),
            entities: Default::default(),
            component_buffers: Default::default(),
        }
    }
}

#[allow(private_interfaces)]
impl SceneCreator for RecycleScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(mut self: Box<Self>, app: &mut App) -> Scene {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size: Vector2<u32> = mint.into();
        self.scene.world_camera2d.resize(window_size);
        self.scene.component_buffers.init(self.component_buffers);
        self.scene.systems.init(&self.systems);
        self.scene.entities.init(&app.globals, self.entities);
        self.scene
    }

    fn systems(&mut self) -> &mut Vec<System> {
        &mut self.systems
    }

    fn entities(&mut self) -> &mut Vec<(EntityScope, Box<RefCell<dyn EntityType>>)> {
        &mut self.entities
    }

    fn components(&mut self) -> &mut FxHashMap<&'static str, Box<dyn ComponentBufferImpl>> {
        &mut self.component_buffers
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
    pub component_buffers: ComponentBufferManager,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "TaskManager::new"))]
    pub tasks: TaskManager,
}

impl Scene {
    pub(crate) fn new(
        id: u32,
        app: &mut App,
        systems: Vec<System>,
        entities: Vec<(EntityScope, Box<RefCell<dyn EntityType>>)>,
        component_buffers: FxHashMap<&'static str, Box<dyn ComponentBufferImpl>>,
    ) -> Self {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size: Vector2<u32> = mint.into();

        let mut scene = Self {
            world_camera2d: WorldCamera2D::new(
                window_size,
                Default::default(),
                WorldCameraScaling::Min(WorldCamera2D::DEFAULT_VERTICAL_CAMERA_FOV),
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
            component_buffers: ComponentBufferManager::new(component_buffers),
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

        scene
    }
}
