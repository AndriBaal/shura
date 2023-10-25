use std::cell::RefCell;

use crate::{
    Component, ComponentConfig, ComponentManager, ComponentType, ComponentTypeImplementation,
    GroupManager, ScreenConfig, App, System, SystemManager, Vector, World,
    WorldCamera, WorldCameraScale, Context,
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
    
    pub fn add_component<C: Component>(mut self, config: ComponentConfig) -> Self {
        self.components.push(Box::new(RefCell::new(ComponentType::<C>::new(config))));
        self
    }
    pub fn add_system(mut self, system: System) -> Self {
        self.systems.push(system);
        self
    }
}

impl SceneCreator for NewScene {
    fn new_id(&self) -> u32 {
        self.id
    }

    fn create(self: Box<Self>, app: &mut App) -> Scene {
        return Scene::new(app, self.id, self.systems, self.components);
    }
}

// /// Add a [Scene] that previously has been removed.
// pub struct RecycleScene<'a> {
//     pub id: u32,
//     pub scene: Scene,
//     pub systems: &'a [System],
// }

// impl<'a> RecycleScene<'a> {
//     pub fn new(id: u32, scene: Scene, systems: &'a [System]) -> RecycleScene {
//         Self { id, scene, systems }
//     }
// }

// impl SceneCreator for RecycleScene {
//     fn new_id(&self) -> u32 {
//         self.id
//     }

//     fn create(mut self: Box<Self>, app: &mut App) -> Scene {
//         let mint: mint::Vector2<u32> = app.window.inner_size().into();
//         let window_size: Vector<u32> = mint.into();
//         self.scene.world_camera.resize(window_size);
//         self.scene.id = self.id;
//         let mut ctx = Context::new(app, &mut self.scene, ContextUse::Update);
//         (self.systems)(&mut ctx);
//         return self.scene;
//     }
// }

#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
/// [states](StateManager) and [camera](WorldCamera) identified by an Id
pub struct Scene {
    pub(crate) id: u32,
    pub render_components: bool,
    pub screen_config: ScreenConfig,
    pub world_camera: WorldCamera,
    pub components: ComponentManager,
    pub groups: GroupManager,
    pub world: World,
    pub(crate) systems: SystemManager,
}

impl Scene {
    pub const DEFAULT_VERTICAL_CAMERA_FOV: f32 = 3.0;
    pub(crate) fn new(
        app: &mut App,
        id: u32,
        systems: Vec<System>,
        components: Vec<Box<RefCell<dyn ComponentTypeImplementation>>>,
    ) -> Self {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size: Vector<u32> = mint.into();

        let mut scene = Self {
            id,
            world_camera: WorldCamera::new(
                Default::default(),
                WorldCameraScale::Min(Self::DEFAULT_VERTICAL_CAMERA_FOV),
                window_size,
            ),
            components: ComponentManager::new(&app.globals, components),
            systems: SystemManager::new(&systems),
            groups: GroupManager::new(),
            screen_config: ScreenConfig::new(),
            render_components: true,
            world: World::new(),
        };

        let (_, mut ctx) = Context::new(app, &mut scene);
        for system in &systems {
            match system {
                System::Setup(setup) => {
                    (setup)(&mut ctx);
                }
                _ => ()
            }
        }

        return scene;
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}
