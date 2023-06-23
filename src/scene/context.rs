use crate::{
    ComponentManager, FrameManager, Gpu, GpuDefaults, Input, Scene, SceneManager, ScreenConfig,
    Shura, StateManager, Vector, WorldCamera,
};

#[cfg(feature = "serde")]
use crate::{ComponentTypeId, SceneSerializer, SerializedComponentStorage, StateTypeId};

#[cfg(feature = "serde")]
use rustc_hash::FxHashMap;

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(feature = "physics")]
use crate::physics::World;

#[cfg(feature = "gui")]
use crate::gui::Gui;

/// Context to communicate with the game engine to access components, scenes, camera, physics and much more.
pub struct Context<'a> {
    // Scene
    pub scene_id: &'a u32,
    pub scene_started: &'a bool,
    pub render_components: &'a mut bool,
    pub screen_config: &'a mut ScreenConfig,
    pub scene_states: &'a mut StateManager,
    pub world_camera: &'a mut WorldCamera,
    pub components: &'a mut ComponentManager,
    #[cfg(feature = "physics")]
    pub world: &'a mut World,

    // Shura
    pub frame: &'a FrameManager,
    pub defaults: &'a GpuDefaults,
    pub input: &'a Input,
    pub gpu: &'a Gpu,
    pub end: &'a mut bool,
    pub scenes: &'a mut SceneManager,
    pub window: &'a mut winit::window::Window,
    pub global_states: &'a mut StateManager,
    #[cfg(feature = "gui")]
    pub gui: &'a mut Gui,
    #[cfg(feature = "audio")]
    pub audio: &'a mut AudioManager,

    // Misc
    pub window_size: Vector<u32>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(shura: &'a mut Shura, scene: &'a mut Scene) -> Context<'a> {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size = mint.into();
        Self {
            // Scene
            scene_id: &scene.id,
            scene_started: &scene.started,
            render_components: &mut scene.render_components,
            screen_config: &mut scene.screen_config,
            world_camera: &mut scene.world_camera,
            components: &mut scene.components,
            scene_states: &mut scene.states,
            #[cfg(feature = "physics")]
            world: &mut scene.world,

            // Shura
            frame: &shura.frame,
            defaults: &shura.defaults,
            input: &shura.input,
            gpu: &shura.gpu,
            end: &mut shura.end,
            scenes: &mut shura.scenes,
            window: &mut shura.window,
            global_states: &mut shura.states,
            #[cfg(feature = "gui")]
            gui: &mut shura.gui,
            #[cfg(feature = "audio")]
            audio: &mut shura.audio,

            // Misc
            window_size,
        }
    }

    #[cfg(feature = "serde")]
    pub fn serialize_scene(
        &mut self,
        mut serialize: impl FnMut(&mut SceneSerializer),
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        let components = &self.components;

        let mut serializer =
            SceneSerializer::new(components, &self.global_states, &self.scene_states);
        (serialize)(&mut serializer);

        #[derive(serde::Serialize)]
        struct Scene<'a> {
            id: u32,
            resized: bool,
            switched: bool,
            started: bool,
            render_components: bool,
            screen_config: &'a ScreenConfig,
            world_camera: &'a WorldCamera,
            components: &'a ComponentManager,
            #[cfg(feature = "physics")]
            world: &'a World,
        }

        #[cfg(feature = "physics")]
        {
            use crate::physics::WorldChanges;
            let (ser_components, ser_scene_state, ser_global_state) = serializer.finish();
            let mut world_cpy = self.world.clone();
            let mut changes = WorldChanges::default();

            for (_, ty) in self.components.types() {
                if !ser_components.contains_key(&ty.component_type_id()) {
                    match &ty.storage {
                        crate::ComponentTypeStorage::Single { component, .. } => {
                            if let Some(component) = component {
                                changes.register_remove(component);
                            }
                        }
                        crate::ComponentTypeStorage::Multiple(multiple) => {
                            for (_, component) in &multiple.components {
                                changes.register_remove(component);
                            }
                        }
                        crate::ComponentTypeStorage::MultipleGroups(groups) => {
                            for (_, group) in groups {
                                for (_, component) in &group.components {
                                    changes.register_remove(component);
                                }
                            }
                        }
                    }
                }
            }
            changes.apply(&mut world_cpy);

            let scene = Scene {
                id: *self.scene_id,
                resized: true,
                switched: true,
                started: true,
                render_components: *self.render_components,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                components: self.components,
                world: &world_cpy,
            };
            let scene: (
                &Scene,
                FxHashMap<ComponentTypeId, SerializedComponentStorage>,
                FxHashMap<StateTypeId, Vec<u8>>,
                FxHashMap<StateTypeId, Vec<u8>>,
            ) = (&scene, ser_components, ser_scene_state, ser_global_state);
            let result = bincode::serialize(&scene);
            return result;
        }

        #[cfg(not(feature = "physics"))]
        {
            let (ser_components, ser_scene_state, ser_global_state) = serializer.finish();
            let scene = Scene {
                id: *self.scene_id,
                resized: true,
                switched: true,
                started: true,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                components: self.components,
                render_components: *self.render_components,
            };
            let scene: (
                &Scene,
                FxHashMap<ComponentTypeId, SerializedComponentStorage>,
                FxHashMap<StateTypeId, Vec<u8>>,
                FxHashMap<StateTypeId, Vec<u8>>,
            ) = (&scene, ser_components, ser_scene_state, ser_global_state);
            let result = bincode::serialize(&scene);
            return result;
        }
    }
}
