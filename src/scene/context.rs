use std::sync::Arc;

use crate::{
    App, ComponentManager, DefaultResources, FrameManager, Gpu, GroupManager, Input, Point2, Scene,
    SceneManager, ScreenConfig, SystemManager, Vector2, World, WorldCamera2D, TaskManager,
};

#[cfg(feature = "serde")]
use crate::{
    serde::{GroupDeserializer, GroupSerializer, SceneSerializer},
    ComponentTypeId, GroupHandle,
};

#[cfg(feature = "serde")]
use rustc_hash::FxHashMap;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(feature = "gui")]
use crate::gui::Gui;

/// Context to communicate with the game engine to access components, scenes, camera, physics and much more.
#[non_exhaustive]
pub struct Context<'a> {
    // Scene
    pub render_components: &'a mut bool,
    pub screen_config: &'a mut ScreenConfig,
    pub world_camera2d: &'a mut WorldCamera2D,
    pub components: &'a mut ComponentManager,
    pub groups: &'a mut GroupManager,
    pub world: &'a mut World,
    pub tasks: &'a mut TaskManager,

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
                render_components: &mut scene.render_components,
                screen_config: &mut scene.screen_config,
                world_camera2d: &mut scene.world_camera2d,
                components: &mut scene.components,
                groups: &mut scene.groups,
                world: &mut scene.world,
                tasks: &mut scene.tasks,

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

    #[cfg(feature = "serde")]
    pub fn serialize_scene(
        &mut self,
        mut serialize: impl FnMut(&mut SceneSerializer),
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        let components = &self.components;
        let mut serializer = SceneSerializer::new(components);
        (serialize)(&mut serializer);

        #[derive(serde::Serialize)]
        struct Scene<'a> {
            render_components: bool,
            screen_config: &'a ScreenConfig,
            world_camera2d: &'a WorldCamera2D,
            groups: &'a GroupManager,
            world: &'a World,
        }

        #[cfg(feature = "physics")]
        {
            let ser_components = serializer.finish();
            let mut world_cpy = self.world.clone();
            for ty in self.components.types_mut() {
                if !ser_components.contains_key(&ty.component_type_id()) {
                    ty.deinit_non_serialized(&mut world_cpy);
                }
            }

            let scene = Scene {
                render_components: *self.render_components,
                screen_config: self.screen_config,
                world_camera2d: self.world_camera2d,
                groups: self.groups,
                world: &world_cpy,
            };
            let scene: (&Scene, FxHashMap<ComponentTypeId, Vec<u8>>) = (&scene, ser_components);
            let result = bincode::serialize(&scene);
            return result;
        }

        #[cfg(not(feature = "physics"))]
        {
            let ser_components = serializer.finish();
            let scene = Scene {
                render_components: *self.render_components,
                screen_config: self.screen_config,
                world_camera2d: self.world_camera2d,
                groups: self.groups,
                world: &self.world,
            };
            let scene: (&Scene, FxHashMap<ComponentTypeId, Vec<u8>>) = (&scene, ser_components);
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
        if let Some(mut ser) = GroupSerializer::new(self.world, self.groups, self.components, group)
        {
            serialize(&mut ser);
            return Some(ser.finish());
        }
        return None;
    }

    #[cfg(feature = "serde")]
    pub fn deserialize_group(&mut self, deserialize: GroupDeserializer) -> GroupHandle {
        deserialize.finish(self)
    }
}
