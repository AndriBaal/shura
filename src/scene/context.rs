use crate::{
    Camera, CameraBuffer, Color, ComponentController, ComponentGroup, ComponentGroupDescriptor,
    ComponentHandle, ComponentManager, ComponentPath, ComponentSet, ComponentSetMut,
    ComponentSetRender, ComponentTypeId, DynamicComponent, FrameManager, Gpu, GpuDefaults,
    GroupFilter, Input, InputEvent, InputTrigger, InstanceBuffer, Isometry, Matrix, Model,
    ModelBuilder, Modifier, RenderConfig, RenderEncoder, RenderTarget, Rotation, Scene,
    SceneCreator, SceneManager, ScreenConfig, Shader, ShaderConfig, Shura, Sprite, SpriteSheet,
    Uniform, Vector, WorldCamera,
};

macro_rules! Where {
    (
    $a:lifetime >= $b:lifetime $(,)?
) => {
        &$b & $a()
    };
}

#[cfg(feature = "serde")]
use crate::ComponentSerializer;

#[cfg(feature = "audio")]
use crate::audio::{Sink, Sound};

#[cfg(feature = "physics")]
use crate::{physics::*, BaseComponent, Point};

use std::any::Any;
#[cfg(feature = "physics")]
use std::{
    cell::{Ref, RefMut},
    ops::{Deref, DerefMut},
};

#[cfg(feature = "gui")]
use crate::gui::GuiContext;

#[cfg(feature = "text")]
use crate::text::{FontBrush, TextDescriptor};

#[cfg(feature = "gamepad")]
use crate::gamepad::*;

use instant::{Duration, Instant};

pub struct ShuraFields<'a> {
    pub frame_manager: &'a FrameManager,
    pub defaults: &'a GpuDefaults,
    pub input: &'a Input,
    pub gpu: &'a Gpu,
    pub end: &'a mut bool,
    pub scene_manager: &'a mut SceneManager,
    pub window: &'a mut winit::window::Window,
    pub global_state: &'a mut Box<dyn Any>,
    #[cfg(feature = "gui")]
    pub gui: GuiContext,
    #[cfg(feature = "audio")]
    pub audio: &'a mut rodio::OutputStream,
    #[cfg(feature = "audio")]
    pub audio_handle: &'a mut rodio::OutputStreamHandle,
}

impl<'a> ShuraFields<'a> {
    pub fn from_shura(shura: &'a mut Shura) -> ShuraFields<'a> {
        Self {
            frame_manager: &shura.frame_manager,
            defaults: &shura.defaults,
            input: &shura.input,
            gpu: &shura.gpu,
            end: &mut shura.end,
            scene_manager: &mut shura.scene_manager,
            window: &mut shura.window,
            global_state: &mut shura.global_state,
            #[cfg(feature = "gui")]
            gui: shura.gui.context(),
            #[cfg(feature = "audio")]
            audio: &mut shura.audio,
            #[cfg(feature = "audio")]
            audio_handle: &mut shura.audio_handle,
        }
    }

    pub fn from_ctx(ctx: &'a mut Context) -> ShuraFields<'a> {
        Self {
            frame_manager: ctx.frame_manager,
            defaults: ctx.defaults,
            input: ctx.input,
            gpu: ctx.gpu,
            end: ctx.end,
            scene_manager: ctx.scene_manager,
            window: ctx.window,
            global_state: ctx.global_state,
            #[cfg(feature = "gui")]
            gui: ctx.gui.clone(),
            #[cfg(feature = "audio")]
            audio: ctx.audio,
            #[cfg(feature = "audio")]
            audio_handle: ctx.audio_handle,
        }
    }
}

/// Context to communicate with the game engine to access components, scenes, camera, physics and many more.
pub struct Context<'a> {
    pub scene_id: &'a u32,
    pub scene_resized: &'a bool,
    pub scene_switched: &'a bool,
    pub screen_config: &'a mut ScreenConfig,
    pub world_camera: &'a mut WorldCamera,
    pub component_manager: &'a mut ComponentManager,

    // Shura
    pub frame_manager: &'a FrameManager,
    pub defaults: &'a GpuDefaults,
    pub input: &'a Input,
    pub gpu: &'a Gpu,
    pub end: &'a mut bool,
    pub scene_manager: &'a mut SceneManager,
    pub window: &'a mut winit::window::Window,
    pub global_state: &'a mut Box<dyn Any>,
    #[cfg(feature = "gui")]
    pub gui: GuiContext,
    #[cfg(feature = "audio")]
    pub audio: &'a mut rodio::OutputStream,
    #[cfg(feature = "audio")]
    pub audio_handle: &'a mut rodio::OutputStreamHandle,
}

impl<'a> Context<'a> {
    pub(crate) fn new(shura: &'a mut Shura, scene: &'a mut Scene) -> Context<'a> {
        Self {
            scene_id: &scene.id,
            scene_resized: &scene.resized,
            scene_switched: &scene.switched,
            screen_config: &mut scene.screen_config,
            world_camera: &mut scene.world_camera,
            component_manager: &mut scene.component_manager,

            // Shura
            frame_manager: &shura.frame_manager,
            defaults: &shura.defaults,
            input: &shura.input,
            gpu: &shura.gpu,
            end: &mut shura.end,
            scene_manager: &mut shura.scene_manager,
            window: &mut shura.window,
            global_state: &mut shura.global_state,
            #[cfg(feature = "gui")]
            gui: shura.gui.context(),
            #[cfg(feature = "audio")]
            audio: &mut shura.audio,
            #[cfg(feature = "audio")]
            audio_handle: &mut shura.audio_handle,
        }
    }

    pub(crate) fn from_fields(shura: ShuraFields<'a>, scene: &'a mut Scene) -> Context<'a> {
        Self {
            scene_id: &scene.id,
            scene_resized: &scene.resized,
            scene_switched: &scene.switched,
            screen_config: &mut scene.screen_config,
            world_camera: &mut scene.world_camera,
            component_manager: &mut scene.component_manager,

            // Shura
            frame_manager: shura.frame_manager,
            defaults: shura.defaults,
            input: shura.input,
            gpu: shura.gpu,
            end: shura.end,
            scene_manager: shura.scene_manager,
            window: shura.window,
            global_state: shura.global_state,
            #[cfg(feature = "gui")]
            gui: shura.gui,
            #[cfg(feature = "audio")]
            audio: shura.audio,
            #[cfg(feature = "audio")]
            audio_handle: shura.audio_handle,
        }
    }

    #[cfg(feature = "physics")]
    pub fn component_from_collider(
        &self,
        collider: &ColliderHandle,
    ) -> Option<(ComponentTypeId, ComponentHandle)> {
        self.component_manager
            .world()
            .component_from_collider(collider)
    }

    pub fn does_group_exist(&self, group: u32) -> bool {
        self.component_manager.does_group_exist(group)
    }

    #[cfg(feature = "serde")]
    pub fn serialize(
        &mut self,
        mut serialize: impl FnMut(&mut ComponentSerializer),
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        let component_manager = &self.component_manager;

        let mut serializer = ComponentSerializer::new(component_manager);
        (serialize)(&mut serializer);

        #[derive(serde::Serialize)]
        struct Scene<'a> {
            id: u32,
            resized: bool,
            switched: bool,
            screen_config: &'a ScreenConfig,
            world_camera: &'a WorldCamera,
            component_manager: &'a mut ComponentManager,
        }

        #[cfg(feature = "physics")]
        {
            use std::mem;
            use rustc_hash::FxHashMap;
            let (components, body_handles) = serializer.finish();
            let mut world = self.component_manager.world.borrow_mut();
            let mut world_cpy = world.clone();
            let mut to_remove = vec![];
            for (body_handle, _body) in world.bodies().iter() {
                if !body_handles.contains(&body_handle) {
                    to_remove.push(body_handle);
                }
            }

            for to_remove in to_remove {
                world_cpy.remove_body(to_remove);
            }

            let old_world = mem::replace(world.deref_mut(), world_cpy);

            drop(world);

            let scene = Scene {
                id: *self.scene_id,
                resized: true,
                switched: true,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                component_manager: self.component_manager,
            };
            let scene: (
                &Scene,
                FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
            ) = (&scene, components);
            let result = bincode::serialize(&scene);

            *self.component_manager.world.borrow_mut() = old_world;

            return result;
        }

        #[cfg(not(feature = "physics"))]
        {
            let components = serializer.finish();
            let scene = Scene {
                id: *self.scene_id,
                resized: true,
                switched: true,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                component_manager: self.component_manager,
            };
            let scene: (
                &Scene,
                FxHashMap<ComponentTypeId, Vec<(u32, Vec<Option<(u32, Vec<u8>)>>)>>,
            ) = (&scene, components);
            let result = bincode::serialize(&scene);
            return result;
        }
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Create
    //////////////////////////////////////////////////////////////////////////////////////////////

    pub fn take_global_state<T: Any>(&mut self) -> Option<Box<T>> {
        let state = std::mem::replace(self.global_state, Box::new(()));
        return state.downcast::<T>().ok();
    }

    pub fn global_state<T: Any>(&self) -> Option<&T> {
        self.global_state.downcast_ref::<T>()
    }

    pub fn global_state_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.global_state.downcast_mut::<T>()
    }

    #[cfg(feature = "physics")]
    pub fn create_joint(
        &mut self,
        component1: &BaseComponent,
        component2: &BaseComponent,
        joint: impl Into<GenericJoint>,
    ) -> ImpulseJointHandle {
        self.component_manager
            .world_mut()
            .create_joint(component1, component2, joint)
    }

    #[cfg(feature = "audio")]
    pub fn create_sink(&self) -> Sink {
        Sink::try_new(&self.audio_handle).unwrap()
    }

    pub fn create_instance_buffer(&self, instances: &[Matrix]) -> InstanceBuffer {
        self.gpu.create_instance_buffer(instances)
    }

    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        self.gpu.create_model(builder)
    }

    pub fn create_sprite(&self, bytes: &[u8]) -> Sprite {
        self.gpu.create_sprite(bytes)
    }

    pub fn create_camera_buffer(&self, camera: &Camera) -> CameraBuffer {
        self.gpu.create_camera_buffer(camera)
    }

    pub fn create_render_target(&self, size: Vector<u32>) -> RenderTarget {
        self.gpu.create_render_target(size)
    }

    pub fn create_sprite_from_image(&self, image: image::DynamicImage) -> Sprite {
        self.gpu.create_sprite_from_image(image)
    }

    pub fn create_empty_sprite(&self, size: Vector<u32>) -> Sprite {
        self.gpu.create_empty_sprite(size)
    }

    pub fn create_sprite_sheet(
        &self,
        bytes: &[u8],
        sprites: Vector<u32>,
        sprite_size: Vector<u32>,
    ) -> SpriteSheet {
        self.gpu.create_sprite_sheet(bytes, sprites, sprite_size)
    }

    #[cfg(feature = "text")]
    pub fn create_font(&self, bytes: &'static [u8]) -> FontBrush {
        self.gpu.create_font(bytes)
    }

    #[cfg(feature = "text")]
    pub fn create_text(&mut self, target_size: Vector<u32>, descriptor: TextDescriptor) -> RenderTarget {
        self.gpu.create_text(self.defaults, target_size, descriptor)
    }

    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        self.gpu.create_uniform(data)
    }

    pub fn create_shader(&self, config: ShaderConfig) -> Shader {
        self.gpu.create_shader(config)
    }

    pub fn create_computed_target<'caller>(
        &self,
        texture_size: Vector<u32>,
        compute: impl for<'any> Fn(&mut RenderEncoder, RenderConfig<'any>, [Where!('caller >= 'any); 0]),
    ) -> RenderTarget {
        self.gpu
            .create_computed_target(&self.defaults, texture_size, compute)
    }

    #[cfg(feature = "audio")]
    pub fn create_sound(&self, sound: &'static [u8]) -> Sound {
        return Sound::new(sound);
    }

    #[cfg(feature = "physics")]
    pub fn create_collider<C: ComponentController>(
        &mut self,
        component: &C,
        collider: impl Into<Collider>,
    ) -> ColliderHandle {
        let body_handle = component
            .base()
            .rigid_body_handle()
            .expect("Cannot add a collider to a component with no RigidBody!");
        let component_handle = *component
            .base()
            .handle()
            .expect("Initialize the component before adding additional colliders!");
        self.component_manager.world_mut().create_collider(
            body_handle,
            component_handle,
            C::IDENTIFIER,
            collider,
        )
    }

    pub fn add_group(&mut self, descriptor: &ComponentGroupDescriptor) {
        self.component_manager.add_group(descriptor);
    }

    pub fn add_component<C: ComponentController>(
        &mut self,
        component: C,
    ) -> (&mut C, ComponentHandle) {
        return self.component_manager.add_component(component);
    }

    pub fn add_component_with_group<C: ComponentController>(
        &mut self,
        group: Option<u32>,
        component: C,
    ) -> (&mut C, ComponentHandle) {
        self.component_manager
            .add_component_with_group(group, component)
    }

    pub fn add_scene(&mut self, scene: impl SceneCreator) {
        let scene = scene.create(ShuraFields::from_ctx(self));
        self.scene_manager.add(scene);
    }

    /// Remove a scene by its id
    pub fn remove_scene(&mut self, id: u32) -> Option<Scene> {
        if let Some(scene) = self.scene_manager.remove(id) {
            return Some(scene);
        }
        return None;
    }

    pub fn remove_component(&mut self, handle: &ComponentHandle) -> Option<DynamicComponent> {
        return self.component_manager.remove_component(handle);
    }

    pub fn remove_components<C: ComponentController>(&mut self, filter: GroupFilter) {
        self.component_manager.remove_components::<C>(filter);
    }

    pub fn remove_group(&mut self, group_id: u32) {
        self.component_manager.remove_group(group_id)
    }

    #[cfg(feature = "physics")]
    pub fn remove_joint(&mut self, joint: ImpulseJointHandle) -> Option<ImpulseJoint> {
        self.component_manager.world_mut().remove_joint(joint)
    }

    #[cfg(feature = "physics")]
    pub fn remove_collider(&mut self, collider_handle: ColliderHandle) -> Option<Collider> {
        self.component_manager
            .world_mut()
            .remove_collider(collider_handle)
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Getter
    //////////////////////////////////////////////////////////////////////////////////////////////

    pub fn render_scale(&self) -> f32 {
        self.screen_config.render_scale()
    }

    #[cfg(feature = "physics")]
    pub fn joint(
        &self,
        joint_handle: ImpulseJointHandle,
    ) -> Option<impl Deref<Target = ImpulseJoint> + '_> {
        Ref::filter_map(self.component_manager.world(), |w| w.joint(joint_handle)).ok()
    }

    #[cfg(feature = "physics")]
    pub fn joint_mut(
        &mut self,
        joint_handle: ImpulseJointHandle,
    ) -> Option<impl DerefMut<Target = ImpulseJoint> + '_> {
        RefMut::filter_map(self.component_manager.world_mut(), |w| {
            w.joint_mut(joint_handle)
        })
        .ok()
    }

    #[cfg(feature = "physics")]
    pub fn collider(
        &self,
        collider_handle: ColliderHandle,
    ) -> Option<impl Deref<Target = Collider> + '_> {
        Ref::filter_map(self.component_manager.world(), |w| {
            w.collider(collider_handle)
        })
        .ok()
    }

    #[cfg(feature = "physics")]
    pub fn collider_mut(
        &mut self,
        collider_handle: ColliderHandle,
    ) -> Option<impl DerefMut<Target = Collider> + '_> {
        RefMut::filter_map(self.component_manager.world_mut(), |w| {
            w.collider_mut(collider_handle)
        })
        .ok()
    }

    #[cfg(feature = "physics")]
    pub fn rigid_body(
        &self,
        rigid_body_handle: RigidBodyHandle,
    ) -> Option<impl Deref<Target = RigidBody> + '_> {
        Ref::filter_map(self.component_manager.world(), |w| {
            w.rigid_body(rigid_body_handle)
        })
        .ok()
    }

    #[cfg(feature = "physics")]
    pub fn rigid_body_mut(
        &mut self,
        rigid_body_handle: RigidBodyHandle,
    ) -> Option<impl DerefMut<Target = RigidBody> + '_> {
        RefMut::filter_map(self.component_manager.world_mut(), |w| {
            w.rigid_body_mut(rigid_body_handle)
        })
        .ok()
    }

    #[cfg(feature = "physics")]
    pub fn world(&self) -> impl Deref<Target = World> + '_ {
        self.component_manager.world()
    }

    #[cfg(feature = "physics")]
    pub fn world_mut(&mut self) -> impl DerefMut<Target = World> + '_ {
        self.component_manager.world_mut()
    }

    pub fn is_pressed(&self, trigger: impl Into<InputTrigger>) -> bool {
        self.input.is_pressed(trigger)
    }

    pub fn is_held(&self, trigger: impl Into<InputTrigger>) -> bool {
        self.input.is_held(trigger)
    }

    pub fn wheel_delta(&self) -> f32 {
        self.input.wheel_delta()
    }

    pub fn held_time(&self, trigger: impl Into<InputTrigger>) -> f32 {
        self.input.held_time(trigger)
    }

    pub fn held_time_duration(&self, trigger: impl Into<InputTrigger>) -> Option<Duration> {
        self.input.held_time_duration(trigger)
    }

    pub fn events(&self) -> impl Iterator<Item = (&InputTrigger, &InputEvent)> {
        self.input.events()
    }

    pub fn event(&self, trigger: impl Into<InputTrigger>) -> Option<&InputEvent> {
        self.input.event(trigger)
    }

    pub const fn modifiers(&self) -> Modifier {
        self.input.modifiers()
    }

    pub fn is_vsync(&self) -> bool {
        self.screen_config.vsync
    }

    pub fn render_size(&self) -> Vector<u32> {
        self.gpu.render_size(self.render_scale())
    }

    pub const fn total_frames(&self) -> u64 {
        self.frame_manager.total_frames()
    }

    pub const fn start_time(&self) -> Instant {
        self.frame_manager.start_time()
    }

    pub const fn update_time(&self) -> Instant {
        self.frame_manager.update_time()
    }

    pub fn now(&self) -> Instant {
        self.frame_manager.now()
    }

    pub fn render_components(&self) -> bool {
        self.component_manager.render_components()
    }

    /// Returns a dimension with the distance from the center of the camera to the right and from the
    /// center to the top.
    pub fn camera_fov(&self) -> Vector<f32> {
        self.world_camera.fov()
    }

    pub fn camera_translation(&self) -> &Vector<f32> {
        self.world_camera.translation()
    }

    pub fn camera_rotation(&self) -> &Rotation<f32> {
        self.world_camera.rotation()
    }

    pub fn camera_position(&self) -> &Isometry<f32> {
        self.world_camera.position()
    }

    pub fn camera_target(&self) -> Option<ComponentHandle> {
        self.world_camera.target()
    }

    pub fn clear_color(&self) -> Option<Color> {
        self.screen_config.clear_color()
    }

    pub fn cursor_camera(&self, camera: &Camera) -> Vector<f32> {
        let window_size = self.window_size();
        self.input.cursor_camera(window_size, camera)
    }

    pub fn cursor_world(&self) -> Vector<f32> {
        self.cursor_camera(&self.world_camera)
    }

    pub fn cursor_relative(&self) -> Vector<f32> {
        self.cursor_camera(self.defaults.relative_camera.camera())
    }

    pub fn cursor_relative_bottom_left(&self) -> Vector<f32> {
        self.cursor_camera(self.defaults.relative_bottom_left_camera.camera())
    }

    pub fn cursor_relative_bottom_right(&self) -> Vector<f32> {
        self.cursor_camera(self.defaults.relative_bottom_right_camera.camera())
    }

    pub fn cursor_relative_top_left(&self) -> Vector<f32> {
        self.cursor_camera(self.defaults.relative_top_left_camera.camera())
    }

    pub fn cursor_relative_top_right(&self) -> Vector<f32> {
        self.cursor_camera(self.defaults.relative_top_right_camera.camera())
    }

    // pub fn cursor_world(&self) -> Vector<f32> {
    //     self.input.cursor_world()
    // }

    pub fn cursor_raw(&self) -> &Vector<u32> {
        self.input.cursor_raw()
    }

    pub fn touches_raw(&self) -> impl Iterator<Item = (&u64, &Vector<u32>)> {
        self.input.touches_raw()
    }

    pub fn window_size(&self) -> Vector<u32> {
        let mint: mint::Vector2<u32> = self.window.inner_size().into();
        return mint.into();
    }

    #[cfg(feature = "physics")]
    pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
        self.component_manager
            .world()
            .intersects_ray(collider_handle, ray, max_toi)
    }

    #[cfg(feature = "physics")]
    pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Vector<f32>) -> bool {
        self.component_manager
            .world()
            .intersects_point(collider_handle, point)
    }

    #[cfg(feature = "physics")]
    pub fn test_filter(
        &self,
        filter: QueryFilter,
        handle: ColliderHandle,
        collider: &Collider,
    ) -> bool {
        self.component_manager
            .world()
            .test_filter(filter, handle, collider)
    }

    #[cfg(feature = "physics")]
    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, f32)> {
        self.component_manager
            .world()
            .cast_ray(ray, max_toi, solid, filter)
    }

    #[cfg(feature = "physics")]
    pub fn cast_shape(
        &self,
        shape: &dyn Shape,
        position: &Isometry<f32>,
        velocity: &Vector<f32>,
        max_toi: f32,
        stop_at_penetration: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, TOI)> {
        self.component_manager.world().cast_shape(
            shape,
            position,
            velocity,
            max_toi,
            stop_at_penetration,
            filter,
        )
    }

    #[cfg(feature = "physics")]
    pub fn cast_ray_and_get_normal(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, RayIntersection)> {
        self.component_manager
            .world()
            .cast_ray_and_get_normal(ray, max_toi, solid, filter)
    }

    #[cfg(feature = "physics")]
    pub fn intersections_with_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle, RayIntersection) -> bool,
    ) {
        self.component_manager
            .world()
            .intersections_with_ray(ray, max_toi, solid, filter, callback)
    }

    #[cfg(feature = "physics")]
    pub fn intersections_with_shape(
        &self,
        shape_pos: &Isometry<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.component_manager
            .world()
            .intersections_with_shape(shape_pos, shape, filter, callback)
    }

    #[cfg(feature = "physics")]
    pub fn intersection_with_shape(
        &self,
        shape_pos: &Isometry<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle)> {
        self.component_manager
            .world()
            .intersection_with_shape(shape_pos, shape, filter)
    }

    #[cfg(feature = "physics")]
    pub fn intersections_with_point(
        &self,
        point: &Point<f32>,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.component_manager
            .world()
            .intersections_with_point(point, filter, callback)
    }

    pub const fn total_time_duration(&self) -> Duration {
        self.frame_manager.total_time_duration()
    }

    pub fn total_time(&self) -> f32 {
        self.frame_manager.total_time()
    }

    pub const fn frame_time_duration(&self) -> Duration {
        self.frame_manager.frame_time_duration()
    }

    pub fn frame_time(&self) -> f32 {
        self.frame_manager.frame_time()
    }

    pub const fn fps(&self) -> u32 {
        self.frame_manager.fps()
    }

    pub fn max_fps(&self) -> Option<u32> {
        self.screen_config.max_fps()
    }

    pub fn max_frame_time(&self) -> Option<Duration> {
        self.screen_config.max_frame_time()
    }

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn window_mut(&mut self) -> &mut winit::window::Window {
        &mut self.window
    }

    pub fn group_mut(&mut self, id: u32) -> Option<&mut ComponentGroup> {
        if let Some(group_index) = self.component_manager.group_index(&id) {
            return self.component_manager.group_mut(*group_index);
        }
        return None;
    }

    pub fn group(&self, id: u32) -> Option<&ComponentGroup> {
        if let Some(group_index) = self.component_manager.group_index(&id) {
            return self.component_manager.group(*group_index);
        }
        return None;
    }

    pub fn active_scene(&self) -> u32 {
        self.scene_manager.active_scene()
    }

    pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
        self.scene_manager.scene_ids()
    }

    pub fn does_scene_exist(&self, name: u32) -> bool {
        self.scene_manager.does_scene_exist(name)
    }

    pub fn active_group_ids(&self) -> &[u32] {
        self.component_manager.active_group_ids()
    }

    pub fn group_ids(&self) -> impl Iterator<Item = &u32> {
        self.component_manager.group_ids()
    }

    pub fn component_dynamic(&self, handle: &ComponentHandle) -> Option<&DynamicComponent> {
        self.component_manager.component_dynamic(handle)
    }

    pub fn component_dynamic_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut DynamicComponent> {
        self.component_manager.component_dynamic_mut(handle)
    }

    pub fn component<C: ComponentController>(&self, handle: &ComponentHandle) -> Option<&C> {
        self.component_manager.component::<C>(handle)
    }

    pub fn component_mut<C: ComponentController>(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut C> {
        self.component_manager.component_mut::<C>(handle)
    }

    pub fn force_buffer<C: ComponentController>(&mut self, filter: GroupFilter) {
        self.component_manager.force_buffer::<C>(filter)
    }

    #[cfg(feature = "physics")]
    pub fn gravity(&self) -> Vector<f32> {
        self.component_manager.world().gravity()
    }

    #[cfg(feature = "physics")]
    pub fn time_scale(&self) -> f32 {
        self.component_manager.world().time_scale()
    }

    #[cfg(feature = "physics")]
    pub fn physics_priority(&self) -> Option<i16> {
        self.component_manager.world().physics_priority()
    }

    pub fn path_render<C: ComponentController>(
        &self,
        path: &ComponentPath<C>,
    ) -> ComponentSetRender<C> {
        return self.component_manager.path_render(path);
    }

    pub fn path<C: ComponentController>(&self, path: &ComponentPath<C>) -> ComponentSet<C> {
        return self.component_manager.path(path);
    }

    pub fn path_mut<C: ComponentController>(
        &mut self,
        path: &ComponentPath<C>,
    ) -> ComponentSetMut<C> {
        return self.component_manager.path_mut(path);
    }

    pub fn components_mut<C: ComponentController>(
        &mut self,
        filter: GroupFilter,
    ) -> ComponentSetMut<C> {
        self.component_manager.components_mut::<C>(filter)
    }

    pub fn components<C: ComponentController>(&self, filter: GroupFilter) -> ComponentSet<C> {
        self.component_manager.components::<C>(filter)
    }

    pub fn first<C: ComponentController>(&self, filter: GroupFilter) -> Option<&C> {
        self.component_manager.first::<C>(filter)
    }

    pub fn first_mut<C: ComponentController>(&mut self, filter: GroupFilter) -> Option<&mut C> {
        self.component_manager.first_mut::<C>(filter)
    }

    #[cfg(feature = "gamepad")]
    pub fn gamepads(&self) -> Option<ConnectedGamepadsIterator> {
        self.input.gamepads()
    }

    #[cfg(feature = "gamepad")]
    pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
        self.input.gamepad(gamepad_id)
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Setter
    //////////////////////////////////////////////////////////////////////////////////////////////
    pub fn set_global_state<T: Any>(&mut self, state: T) {
        *self.global_state = Box::new(state)
    }

    pub fn set_render_scale(&mut self, scale: f32) {
        self.screen_config.set_render_scale(scale);
    }

    pub fn set_active_scene(&mut self, active_scene: u32) {
        self.scene_manager.set_active_scene(active_scene)
    }

    pub fn set_render_components(&mut self, render_components: bool) {
        self.component_manager
            .set_render_components(render_components)
    }

    pub fn set_camera_position(&mut self, pos: Isometry<f32>) {
        self.world_camera.set_position(pos);
    }

    pub fn set_camera_translation(&mut self, translation: Vector<f32>) {
        self.world_camera.set_translation(translation);
    }

    pub fn set_camera_rotation(&mut self, rotation: Rotation<f32>) {
        self.world_camera.set_rotation(rotation);
    }

    pub fn set_camera_target(&mut self, target: Option<ComponentHandle>) {
        self.world_camera.set_target(target);
    }

    /// Tries to enable or disable vSync. The default is always vSync to be on.
    /// So every device supports vSync but not every device supports no vSync.
    pub fn set_vsync(&mut self, vsync: bool) {
        self.screen_config.set_vsync(vsync);
    }

    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.window.set_cursor_visible(!hidden);
    }

    /// Set the distance between the center of the camera to the top in world coordinates.
    pub fn set_camera_vertical_fov(&mut self, fov: f32) {
        self.world_camera.set_vertical_fov(fov);
    }

    /// Set the distance between the center of the camera to the right in world coordinates.
    pub fn set_camera_horizontal_fov(&mut self, fov: f32) {
        self.world_camera.set_horizontal_fov(fov);
    }

    pub fn set_window_size(&mut self, size: Vector<u32>) {
        let mint: mint::Vector2<u32> = size.into();
        let size: winit::dpi::PhysicalSize<u32> = mint.into();
        self.window.set_inner_size(size);
    }

    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if fullscreen {
            let f = winit::window::Fullscreen::Borderless(None);
            self.window.set_fullscreen(Some(f));
        } else {
            self.window.set_fullscreen(None);
        }
    }

    pub fn set_clear_color(&mut self, color: Option<Color>) {
        self.screen_config.set_clear_color(color);
    }

    pub fn set_window_resizable(&mut self, resizable: bool) {
        self.window.set_resizable(resizable);
    }

    pub fn set_window_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    #[cfg(feature = "physics")]
    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.component_manager.world_mut().set_gravity(gravity);
    }

    #[cfg(feature = "physics")]
    pub fn set_time_scale(&mut self, time_scale: f32) {
        self.component_manager
            .world_mut()
            .set_time_scale(time_scale);
    }

    #[cfg(feature = "physics")]
    pub fn set_physics_priority(&mut self, step: Option<i16>) {
        self.component_manager
            .world_mut()
            .set_physics_priority(step);
    }

    pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
        self.screen_config.set_max_fps(max_fps);
    }
}
