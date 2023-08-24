use std::sync::Arc;

use crate::{
    ComponentManager, FrameManager, Gpu, GpuDefaults, GroupManager, Input, Scene, SceneManager,
    ScreenConfig, Shura, Vector, World, WorldCamera,
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

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq)]
pub(crate) enum ContextUse {
    Render,
    Update,
}

/// Context to communicate with the game engine to access components, scenes, camera, physics and much more.
#[non_exhaustive]
pub struct Context<'a> {
    // Scene
    pub scene_id: &'a u32,
    pub scene_started: &'a bool,
    pub update_components: &'a mut i16,
    pub render_components: &'a mut bool,
    pub screen_config: &'a mut ScreenConfig,
    pub world_camera: &'a mut WorldCamera,
    pub components: &'a mut ComponentManager,
    pub groups: &'a mut GroupManager,
    pub world: &'a mut World,

    // Shura
    pub frame: &'a FrameManager,
    pub defaults: &'a GpuDefaults,
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
    pub window_size: Vector<u32>,
    pub cursor: Vector<f32>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        shura: &'a mut Shura,
        scene: &'a mut Scene,
        context_use: ContextUse,
    ) -> Context<'a> {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size = mint.into();
        let cursor = shura.input.cursor(&scene.world_camera);
        Self {
            // Scene
            scene_id: &scene.id,
            scene_started: &scene.started,
            render_components: &mut scene.render_components,
            update_components: &mut scene.update_components,
            screen_config: &mut scene.screen_config,
            world_camera: &mut scene.world_camera,
            components: scene.components.with_use(context_use),
            groups: &mut scene.groups,
            world: &mut scene.world,

            // Shura
            frame: &shura.frame,
            defaults: &shura.defaults,
            input: &shura.input,
            gpu: shura.gpu.clone(),
            #[cfg(feature = "gui")]
            gui: &mut shura.gui,
            #[cfg(feature = "audio")]
            audio: &shura.audio,
            end: &mut shura.end,
            scenes: &mut shura.scenes,
            window: &mut shura.window,

            // Misc
            window_size,
            cursor,
        }
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
            id: u32,
            update_components: i16,
            started: bool,
            render_components: bool,
            screen_config: &'a ScreenConfig,
            world_camera: &'a WorldCamera,
            components: &'a ComponentManager,
            groups: &'a GroupManager,
            world: &'a World,
        }

        #[cfg(feature = "physics")]
        {
            let ser_components = serializer.finish();
            let mut world_cpy = self.world.clone();
            for mut ty in self.components.types_mut() {
                if !ser_components.contains_key(&ty.component_type_id()) {
                    ty.deinit_non_serialized(&mut world_cpy);
                }
            }

            let scene = Scene {
                id: *self.scene_id,
                started: true,
                render_components: *self.render_components,
                update_components: *self.update_components,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                components: self.components,
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
                id: *self.scene_id,
                started: true,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                components: self.components,
                groups: self.groups,
                render_components: *self.render_components,
                update_components: *self.update_components,
                world: self.world,
            };
            let scene: (
                &Scene,
                FxHashMap<ComponentTypeId, SerializedComponentStorage>,
            ) = (&scene, ser_components);
            let result = bincode::serialize(&scene);
            return result;
        }
    }

    #[cfg(feature = "serde")]
    pub fn serialize_group(
        &self,
        group: GroupHandle,
        serialize: impl FnOnce(&mut GroupSerializer),
    ) -> Vec<u8> {
        let mut ser = GroupSerializer::new(group, self.components);
        serialize(&mut ser);
        return ser.finish(&self.groups);
    }

    // #[cfg(feature = "serde")]
    // pub fn deserialize_group(&self, deserialize: GroupDeserializer) -> GroupHandle {

    // }
}
