use std::sync::Arc;

use crate::{
    entity::{EntityManager, GroupManager},
    graphics::{
        ComponentBufferManager, DefaultResources, Gpu, ScreenConfig, WorldCamera2D, WorldCamera3D,
    },
    input::Input,
    math::{Point2, Vector2},
    physics::World,
    prelude::{App, FrameManager, Scene, SceneManager},
    system::SystemManager,
    tasks::TaskManager,
};
#[cfg(feature = "serde")]
use crate::{
    entity::{EntityTypeId, GroupHandle},
    serde::{GroupDeserializer, GroupSerializer, SceneSerializer},
};

#[cfg(feature = "serde")]
use rustc_hash::FxHashMap;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(feature = "gui")]
use crate::gui::Gui;

#[non_exhaustive]
pub struct Context<'a> {
    // Scene
    pub render_entities: &'a mut bool,
    pub screen_config: &'a mut ScreenConfig,
    pub world_camera2d: &'a mut WorldCamera2D,
    pub world_camera3d: &'a mut WorldCamera3D,
    pub entities: &'a mut EntityManager,
    pub groups: &'a mut GroupManager,
    pub world: &'a mut World,
    pub tasks: &'a mut TaskManager,
    pub component_buffers: &'a mut ComponentBufferManager,

    // App
    pub frame: &'a FrameManager,
    pub defaults: &'a DefaultResources,
    pub input: &'a Input,
    pub gpu: Arc<Gpu>,
    #[cfg(feature = "gui")]
    pub gui: &'a mut Gui,
    #[cfg(feature = "audio")]
    pub audio: &'a AudioManager,
    pub end: &'a mut bool,
    pub scenes: &'a mut SceneManager,
    pub window: &'a mut winit::window::Window,

    // Misc
    pub scene_id: &'a u32,
    pub window_size: Vector2<u32>,
    pub cursor: Point2<f32>,
    pub resized: bool,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        scene_id: &'a u32,
        app: &'a mut App,
        scene: &'a mut Scene,
    ) -> (&'a mut SystemManager, Context<'a>) {
        let mint: mint::Vector2<u32> = app.window.inner_size().into();
        let window_size = mint.into();
        let cursor = app.input.cursor(&scene.world_camera2d);
        (
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
                component_buffers: &mut scene.component_buffers,

                // App
                frame: &app.frame,
                defaults: &app.defaults,
                input: &app.input,
                gpu: app.gpu.clone(),
                #[cfg(feature = "gui")]
                gui: &mut app.gui,
                #[cfg(feature = "audio")]
                audio: &app.audio,
                end: &mut app.end,
                scenes: &mut app.scenes,
                window: &mut app.window,

                // Misc
                scene_id,
                window_size,
                cursor,
                resized: app.resized,
            },
        )
    }

    // pub(crate) fn new_from_ctx(
    //     scene_id: &'a u32,
    //     ctx: Context<'a>,
    //     scene: &'a mut Scene,
    // ) -> (&'a mut SystemManager, Context<'a>) {
    //     let mint: mint::Vector2<u32> = ctx.window.inner_size().into();
    //     let window_size = mint.into();
    //     let cursor = ctx.input.cursor(&scene.world_camera2d);
    //     (
    //         &mut scene.systems,
    //         Self {
    //             // Scene
    //             render_entities: &mut scene.render_entities,
    //             screen_config: &mut scene.screen_config,
    //             world_camera2d: &mut scene.world_camera2d,
    //             world_camera3d: &mut scene.world_camera3d,
    //             entities: &mut scene.entities,
    //             groups: &mut scene.groups,
    //             world: &mut scene.world,
    //             tasks: &mut scene.tasks,
    //             component_buffers: &mut scene.component_buffers,

    //             // Misc
    //             scene_id,
    //             window_size,
    //             cursor,
    //             resized: false,

    //             ..ctx
    //         },
    //     )
    // }

    #[cfg(feature = "serde")]
    pub fn serialize_scene(
        &mut self,
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
            groups: &'a GroupManager,
            world: &'a World,
        }

        #[cfg(feature = "physics")]
        {
            let ser_entities = serializer.finish();
            let mut world_cpy = self.world.clone();
            for ty in self.entities.types_mut() {
                if !ser_entities.contains_key(&ty.entity_type_id()) {
                    for entity in ty.iter_dyn() {
                        for component in entity.components_dyn() {
                            world_cpy.remove_no_maintain(component);
                        }
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
            let scene: (&Scene, FxHashMap<EntityTypeId, Vec<u8>>) = (&scene, ser_entities);

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
            let scene: (&Scene, FxHashMap<EntityTypeId, Vec<u8>>) = (&scene, ser_entities);
            let result = bincode::serialize(&scene);
            return result;
        }
    }

    #[cfg(feature = "serde")]
    pub fn serialize_group(
        &mut self,
        group: GroupHandle,
        serialize: impl FnOnce(&mut GroupSerializer),
    ) -> Option<Result<Vec<u8>, Box<bincode::ErrorKind>>> {
        if let Some(mut ser) = GroupSerializer::new(self.world, self.groups, self.entities, group) {
            serialize(&mut ser);
            return Some(ser.finish());
        }
        None
    }

    #[cfg(feature = "serde")]
    pub fn deserialize_group(&mut self, deserialize: GroupDeserializer) -> GroupHandle {
        deserialize.finish(self)
    }
}
