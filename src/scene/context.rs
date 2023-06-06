use crate::{
    ComponentManager, FrameManager, GlobalStateManager, Gpu, GpuDefaults, Input, Scene,
    SceneManager, SceneStateManager, ScreenConfig, Shura, Vector, WorldCamera,
};

#[cfg(feature = "serde")]
use crate::{ComponentTypeId, GroupHandle, SceneSerializer, StateTypeId};

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(feature = "physics")]
use crate::physics::World;

#[cfg(feature = "gui")]
use crate::gui::Gui;

// pub struct ShuraFields<'a> {
//     pub frame: &'a FrameManager,
//     pub defaults: &'a GpuDefaults,
//     pub input: &'a Input,
//     pub gpu: &'a Gpu,
//     pub end: &'a mut bool,
//     pub scenes: &'a mut SceneManager,
//     pub window: &'a mut winit::window::Window,
//     pub states: &'a mut GlobalStateManager,
//     #[cfg(feature = "gui")]
//     pub gui: &'a mut Gui,
//     #[cfg(feature = "audio")]
//     pub audio: &'a mut AudioManager,
// }

// impl<'a> ShuraFields<'a> {
//     pub(crate) fn from_shura(shura: &'a mut Shura) -> ShuraFields<'a> {
//         Self {
//             frame: &shura.frame,
//             defaults: &shura.defaults,
//             input: &shura.input,
//             gpu: &shura.gpu,
//             end: &mut shura.end,
//             scenes: &mut shura.scenes,
//             window: &mut shura.window,
//             states: &mut shura.states,
//             #[cfg(feature = "gui")]
//             gui: &mut shura.gui,
//             #[cfg(feature = "audio")]
//             audio: &mut shura.audio,
//         }
//     }

//     pub fn from_ctx(ctx: &'a mut Context) -> ShuraFields<'a> {
//         Self {
//             frame: ctx.frame,
//             defaults: ctx.defaults,
//             input: ctx.input,
//             gpu: ctx.gpu,
//             end: ctx.end,
//             scenes: ctx.scenes,
//             window: ctx.window,
//             states: ctx.global_states,
//             #[cfg(feature = "gui")]
//             gui: ctx.gui,
//             #[cfg(feature = "audio")]
//             audio: ctx.audio,
//         }
//     }
// }

/// Context to communicate with the game engine to access components, scenes, camera, physics and much more.
/// The Context provides easy access to the most common methods. Some methods are not present in the
/// implementation of the Context, but are inside one of Context's underlying fields (You might also
/// need to access the underlying fields to avoid borrow issues).
pub struct Context<'a> {
    // Scene
    pub scene_id: &'a u32,
    pub scene_resized: &'a bool,
    pub scene_switched: &'a bool,
    pub scene_started: &'a bool,
    pub render_components: &'a mut bool,
    pub screen_config: &'a mut ScreenConfig,
    pub scene_states: &'a mut SceneStateManager,
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
    pub global_states: &'a mut GlobalStateManager,
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
            scene_resized: &scene.resized,
            scene_started: &scene.started,
            scene_switched: &scene.switched,
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

    // pub(crate) fn from_fields(shura: ShuraFields<'a>, scene: &'a mut Scene) -> Context<'a> {
    //     let mint: mint::Vector2<u32> = shura.window.inner_size().into();
    //     let window_size = mint.into();
    //     Self {
    //         scene_id: &scene.id,
    //         scene_resized: &scene.resized,
    //         scene_started: &scene.started,
    //         scene_switched: &scene.switched,
    //         render_components: &mut scene.render_components,
    //         screen_config: &mut scene.screen_config,
    //         world_camera: &mut scene.world_camera,
    //         components: &mut scene.components,
    //         scene_states: &mut scene.states,
    //         #[cfg(feature = "physics")]
    //         world: &mut scene.world,

    //         // Shura
    //         frame: shura.frame,
    //         defaults: shura.defaults,
    //         input: shura.input,
    //         gpu: shura.gpu,
    //         end: shura.end,
    //         scenes: shura.scenes,
    //         window: shura.window,
    //         global_states: shura.states,
    //         #[cfg(feature = "gui")]
    //         gui: shura.gui,
    //         #[cfg(feature = "audio")]
    //         audio: shura.audio,

    //         window_size,
    //     }
    // }

    #[cfg(feature = "serde")]
    pub fn serialize_scene(
        &mut self,
        mut serialize: impl FnMut(&mut SceneSerializer),
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        use rustc_hash::FxHashMap;

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
                    for (_, group) in &ty.groups {
                        for (_, component) in &group.components {
                            changes.register_remove(component);
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
                FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
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
                FxHashMap<ComponentTypeId, Vec<(GroupHandle, Vec<Option<(u32, Vec<u8>)>>)>>,
                FxHashMap<StateTypeId, Vec<u8>>,
                FxHashMap<StateTypeId, Vec<u8>>,
            ) = (&scene, ser_components, ser_scene_state, ser_global_state);
            let result = bincode::serialize(&scene);
            return result;
        }
    }

    // pub fn remove_scene(&mut self, id: u32) -> Option<Scene> {
    //     if let Some(mut scene) = self.scenes.remove(id) {
    //         for end in scene.states.ends() {
    //             let mut ctx = Context::from_fields(ShuraFields::from_ctx(self), &mut scene);
    //             end(&mut ctx);
    //         }
    //         return Some(scene);
    //     }
    //     return None;
    // }

    // pub fn add_scene(&mut self, scene: impl SceneCreator) {
    //     let scene = scene.scene(ShuraFields::from_ctx(self));
    //     self.scenes.add(scene);
    // }
}
