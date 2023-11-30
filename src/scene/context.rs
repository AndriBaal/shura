use std::{cell::Ref, sync::Arc};

use crate::{
    App, CameraBuffer, CameraBuffer2D, ComponentBufferManager, DefaultResources, Entity,
    EntityManager, EntitySet, FrameManager, Gpu, GroupFilter, GroupManager, Input, Instance2D,
    InstanceBuffer, InstanceIndices, Mesh2D, Point2, Renderer, Scene, SceneManager, ScreenConfig,
    SystemManager, TaskManager, Vector2, World, WorldCamera2D, WorldCamera3D,
};

#[cfg(feature = "serde")]
use crate::{
    serde::{GroupDeserializer, GroupSerializer, SceneSerializer},
    EntityTypeId, GroupHandle,
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

    #[cfg(feature = "serde")]
    pub fn serialize_scene(
        &mut self,
        mut serialize: impl FnMut(&mut SceneSerializer),
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        let entities = &self.entities;
        let mut serializer = SceneSerializer::new(entities);
        (serialize)(&mut serializer);

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
                    ty.deinit_non_serialized(&mut world_cpy);
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
            let scene: (&Scene, FxHashMap<ComponentTypeId, Vec<u8>>) = (&scene, ser_entities);
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

pub struct RenderContext<'a> {
    entities: &'a EntityManager,
    pub component_buffers: &'a ComponentBufferManager,

    pub world_camera2d: &'a CameraBuffer2D,
    pub world_camera3d: &'a CameraBuffer<WorldCamera3D>,
    pub relative_camera: &'a CameraBuffer2D,
    pub relative_bottom_left_camera: &'a CameraBuffer2D,
    pub relative_bottom_right_camera: &'a CameraBuffer2D,
    pub relative_top_left_camera: &'a CameraBuffer2D,
    pub relative_top_right_camera: &'a CameraBuffer2D,
    pub unit_camera: &'a CameraBuffer2D,
    pub unit_mesh: &'a Mesh2D,
    pub centered_instance: &'a InstanceBuffer<Instance2D>,
}

impl<'a> RenderContext<'a> {
    pub(crate) fn new(
        defaults: &'a DefaultResources,
        scene: &'a Scene,
    ) -> (&'a SystemManager, Self) {
        (
            &scene.systems,
            Self {
                entities: &scene.entities,
                component_buffers: &scene.component_buffers,
                relative_camera: &defaults.relative_camera.0,
                relative_bottom_left_camera: &defaults.relative_bottom_left_camera.0,
                relative_bottom_right_camera: &defaults.relative_bottom_right_camera.0,
                relative_top_left_camera: &defaults.relative_top_left_camera.0,
                relative_top_right_camera: &defaults.relative_top_right_camera.0,
                unit_camera: &defaults.unit_camera.0,
                centered_instance: &defaults.centered_instance,
                unit_mesh: &defaults.unit_mesh,
                world_camera2d: &defaults.world_camera2d,
                world_camera3d: &defaults.world_camera3d,
            },
        )
    }

    #[inline]
    pub fn set<E: Entity>(&'a self) -> EntitySet<'a, E> {
        self.set_of(GroupFilter::Active)
    }

    pub fn set_of<E: Entity>(&'a self, filter: GroupFilter<'a>) -> EntitySet<'a, E> {
        self.entities.set_ref_of(filter)
    }

    pub fn single<E: Entity>(&self) -> Ref<E> {
        self.entities.single_ref::<E>()
    }

    pub fn try_single<E: Entity>(&self) -> Option<Ref<E>> {
        self.entities.try_single_ref()
    }

    pub fn render_all<I: crate::Instance>(
        &self,
        renderer: &mut Renderer<'a>,
        name: &'static str,
        all: impl Fn(&mut Renderer<'a>, &'a InstanceBuffer<I>, InstanceIndices),
    ) {
        let buffer = self
            .component_buffers
            .get::<I>(name)
            .unwrap_or_else(|| panic!("Component {name} is not registered!"))
            .buffer();

        if buffer.instance_amount() != 0 {
            (all)(renderer, buffer, buffer.instances());
        }
    }
}
