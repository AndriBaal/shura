use std::{cell::RefCell, sync::Arc};

#[cfg(feature = "serde")]
use rustc_hash::FxHashMap;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;
#[cfg(feature = "gui")]
use crate::gui::Gui;
#[cfg(feature = "serde")]
use crate::{
    entity::{EntityGroupHandle, EntityId},
    serde::{EntityGroupDeserializer, EntityGroupSerializer, SceneSerializer},
};
use crate::{
    entity::{EntityGroupManager, EntityManager},
    graphics::{Gpu, RenderGroupManager, ScreenConfig, WorldCamera2D, WorldCamera3D},
    input::Input,
    io::{AssetManager, StorageManager},
    math::{Point2, Vector2},
    physics::World,
    prelude::{App, Scene, SceneManager, TimeManager, WindowEventManager},
    system::{EndReason, SystemManager},
    tasks::TaskManager,
};

#[non_exhaustive]
pub struct Context<'a> {
    // Scene
    pub render_entities: &'a mut bool,
    pub screen_config: &'a mut ScreenConfig,
    pub world_camera2d: &'a mut WorldCamera2D,
    pub world_camera3d: &'a mut WorldCamera3D,
    pub entities: &'a mut EntityManager,
    pub groups: &'a mut EntityGroupManager,
    pub world: &'a mut World,
    pub tasks: &'a mut TaskManager,
    pub render_groups: &'a mut RenderGroupManager,
    pub started: &'a bool,

    // App
    pub time: &'a TimeManager,
    pub input: &'a Input,
    pub gpu: Arc<Gpu>,
    #[cfg(feature = "gui")]
    pub gui: &'a mut Gui,
    #[cfg(feature = "audio")]
    pub audio: &'a AudioManager,
    pub end: &'a mut bool,
    pub scenes: &'a mut SceneManager,
    pub window: Arc<winit::window::Window>,
    pub event_loop: &'a winit::event_loop::ActiveEventLoop,
    pub storage: Arc<dyn StorageManager>,
    pub assets: Arc<dyn AssetManager>,

    // Misc
    pub scene_id: &'a u32,
    pub surface_size: Vector2<u32>,
    pub render_size: Vector2<u32>,
    pub cursor: Point2<f32>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        scene_id: &'a u32,
        app: &'a mut App,
        scene: &'a mut Scene,
        event_loop: &'a winit::event_loop::ActiveEventLoop,
    ) -> (&'a mut WindowEventManager, &'a mut SystemManager, Context<'a>) {
        let surface_size = app.gpu.surface_size();
        let render_size = scene.screen_config.render_size(&app.gpu);

        let cursor = app.input.cursor(&scene.world_camera2d);
        (
            &mut app.window_events,
            &mut scene.systems,
            Self {
                // Scene
                render_entities: &mut scene.render_entities,
                screen_config: &mut scene.screen_config,
                world_camera2d: &mut scene.world_camera2d,
                world_camera3d: &mut scene.world_camera3d,
                entities: &mut scene.entities,
                groups: &mut scene.groups,
                world: &mut scene.world,
                tasks: &mut scene.tasks,
                render_groups: &mut scene.render_groups,
                started: &scene.started,

                // App
                time: &app.time,
                input: &app.input,
                gpu: app.gpu.clone(),
                storage: app.storage.clone(),
                assets: app.assets.clone(),
                #[cfg(feature = "gui")]
                gui: &mut app.gui,
                #[cfg(feature = "audio")]
                audio: &app.audio,
                end: &mut app.end,
                scenes: &mut app.scenes,
                window: app.window.clone(),
                event_loop,

                // Misc
                scene_id,
                surface_size,
                render_size,
                cursor,
            },
        )
    }

    #[cfg(feature = "serde")]
    #[must_use]
    pub fn serialize_scene(
        &mut self, // Not actually needed, just to ensure there are only unique references to entities
        serialize: impl FnOnce(SceneSerializer) -> SceneSerializer,
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        let entities = &self.entities;
        let serializer = (serialize)(SceneSerializer::new(entities));

        #[derive(serde::Serialize)]
        struct Scene<'a> {
            render_entities: bool,
            screen_config: &'a ScreenConfig,
            world_camera2d: &'a WorldCamera2D,
            world_camera3d: &'a WorldCamera3D,
            groups: &'a EntityGroupManager,
            world: &'a World,
        }

        #[cfg(feature = "physics")]
        {
            let ser_entities = serializer.finish();
            let mut world_cpy = self.world.clone();
            for ty in self.entities.entities() {
                if !ser_entities.contains_key(&ty.entity_type_id()) {
                    for (_, entity) in ty.dyn_iter() {
                        entity.remove_from_world(&mut world_cpy);
                        // for component in entity.components() {
                        //     component.remove_from_world(&mut world_cpy);
                        // }
                    }
                }
            }

            let scene = Scene {
                render_entities: *self.render_entities,
                screen_config: self.screen_config,
                world_camera2d: self.world_camera2d,
                world_camera3d: self.world_camera3d,
                groups: self.groups,
                world: &world_cpy,
            };
            let scene: (&Scene, FxHashMap<EntityId, Vec<u8>>) = (&scene, ser_entities);

            bincode::serialize(&scene)
        }

        #[cfg(not(feature = "physics"))]
        {
            let ser_entities = serializer.finish();
            let scene = Scene {
                render_entities: *self.render_entities,
                screen_config: self.screen_config,
                world_camera2d: self.world_camera2d,
                world_camera3d: self.world_camera3d,
                groups: self.groups,
                world: &self.world,
            };
            let scene: (&Scene, FxHashMap<EntityId, Vec<u8>>) = (&scene, ser_entities);
            let result = bincode::serialize(&scene);
            return result;
        }
    }

    pub fn with_scene(
        &mut self,
        scene_id: u32,
        action: impl FnOnce(&mut SystemManager, &mut Context),
    ) {
        if let Some(scene_rc) = self.scenes.get(scene_id) {
            let scene_cell: &RefCell<_> = &scene_rc;
            let mut scene_ref = scene_cell.borrow_mut();
            let scene = &mut *scene_ref;

            let cursor = self.input.cursor(&scene.world_camera2d);
            let mut ctx = Context {
                // Scene
                render_entities: &mut scene.render_entities,
                screen_config: &mut scene.screen_config,
                world_camera2d: &mut scene.world_camera2d,
                world_camera3d: &mut scene.world_camera3d,
                entities: &mut scene.entities,
                groups: &mut scene.groups,
                world: &mut scene.world,
                tasks: &mut scene.tasks,
                render_groups: &mut scene.render_groups,
                started: &scene.started,

                // Misc
                scene_id: &scene_id,
                surface_size: self.surface_size,
                render_size: self.render_size,
                cursor,

                time: self.time,
                input: self.input,
                gpu: self.gpu.clone(),
                storage: self.storage.clone(),
                assets: self.assets.clone(),
                #[cfg(feature = "gui")]
                gui: self.gui,
                #[cfg(feature = "audio")]
                audio: self.audio,
                end: self.end,
                scenes: self.scenes,
                window: self.window.clone(),
                event_loop: self.event_loop,
            };
            (action)(&mut scene.systems, &mut ctx);
        }
    }

    pub fn add_scene(&mut self, scene_id: u32, scene: impl Into<Scene>) {
        self.scenes.add(scene_id, scene);
    }

    #[must_use]
    pub fn remove_scene(&mut self, scene_id: u32) -> Option<Scene> {
        if self.scenes.exists(scene_id) {
            self.with_scene(scene_id, |systems, ctx| {
                for (_, setup) in &systems.end_systems {
                    (setup)(ctx, EndReason::Removed);
                }
            });
        }

        self.scenes.remove(scene_id)
    }

    #[cfg(feature = "serde")]
    pub fn serialize_group(
        &mut self,
        group: &EntityGroupHandle,
        serialize: impl FnOnce(&mut EntityGroupSerializer),
    ) -> Option<Result<Vec<u8>, Box<bincode::ErrorKind>>> {
        if let Some(mut ser) =
            EntityGroupSerializer::new(self.world, self.groups, self.entities, group)
        {
            serialize(&mut ser);
            return Some(ser.finish());
        }
        None
    }

    #[cfg(feature = "serde")]
    pub fn deserialize_group(&mut self, deserialize: EntityGroupDeserializer) -> EntityGroupHandle {
        deserialize.finish(self)
    }
}
