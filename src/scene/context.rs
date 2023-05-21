use crate::{
    ComponentManager, FrameManager, 
    GlobalStateManager, Gpu, GpuDefaults, Input, Scene,
    SceneCreator, SceneManager, SceneStateManager, ScreenConfig, Shura, Vector, WorldCamera,
};

#[cfg(feature = "serde")]
use crate::{SceneSerializer, StateTypeId};

#[cfg(feature = "audio")]
use crate::audio::AudioManager;

#[cfg(feature = "physics")]
use crate::{physics::World};

#[cfg(feature = "gui")]
use crate::gui::Gui;

pub struct ShuraFields<'a> {
    pub frame: &'a FrameManager,
    pub defaults: &'a GpuDefaults,
    pub input: &'a Input,
    pub gpu: &'a Gpu,
    pub end: &'a mut bool,
    pub scenes: &'a mut SceneManager,
    pub window: &'a mut winit::window::Window,
    pub states: &'a mut GlobalStateManager,
    #[cfg(feature = "gui")]
    pub gui: &'a mut Gui,
    #[cfg(feature = "audio")]
    pub audio: &'a mut AudioManager,
}

impl<'a> ShuraFields<'a> {
    pub(crate) fn from_shura(shura: &'a mut Shura) -> ShuraFields<'a> {
        Self {
            frame: &shura.frame,
            defaults: &shura.defaults,
            input: &shura.input,
            gpu: &shura.gpu,
            end: &mut shura.end,
            scenes: &mut shura.scenes,
            window: &mut shura.window,
            states: &mut shura.states,
            #[cfg(feature = "gui")]
            gui: &mut shura.gui,
            #[cfg(feature = "audio")]
            audio: &mut shura.audio,
        }
    }

    pub fn from_ctx(ctx: &'a mut Context) -> ShuraFields<'a> {
        Self {
            frame: ctx.frame,
            defaults: ctx.defaults,
            input: ctx.input,
            gpu: ctx.gpu,
            end: ctx.end,
            scenes: ctx.scenes,
            window: ctx.window,
            states: ctx.global_states,
            #[cfg(feature = "gui")]
            gui: ctx.gui,
            #[cfg(feature = "audio")]
            audio: ctx.audio,
        }
    }
}

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

    pub(crate) fn from_fields(shura: ShuraFields<'a>, scene: &'a mut Scene) -> Context<'a> {
        let mint: mint::Vector2<u32> = shura.window.inner_size().into();
        let window_size = mint.into();
        Self {
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
            frame: shura.frame,
            defaults: shura.defaults,
            input: shura.input,
            gpu: shura.gpu,
            end: shura.end,
            scenes: shura.scenes,
            window: shura.window,
            global_states: shura.states,
            #[cfg(feature = "gui")]
            gui: shura.gui,
            #[cfg(feature = "audio")]
            audio: shura.audio,

            window_size,
        }
    }

    #[cfg(feature = "serde")]
    pub fn serialize_scene(
        &mut self,
        filter: ComponentFilter,
        mut serialize: impl FnMut(&mut SceneSerializer),
    ) -> Result<Vec<u8>, Box<bincode::ErrorKind>> {
        use rustc_hash::FxHashMap;

        let components = &self.components;

        let mut serializer =
            SceneSerializer::new(components, &self.global_states, &self.scene_states, filter);
        (serialize)(&mut serializer);

        #[derive(serde::Serialize)]
        struct Scene<'a> {
            id: u32,
            resized: bool,
            switched: bool,
            started: bool,
            screen_config: &'a ScreenConfig,
            world_camera: &'a WorldCamera,
            components: &'a ComponentManager,
        }

        #[cfg(feature = "physics")]
        {
            use std::mem;
            let (groups, ser_components, ser_scene_state, ser_global_state, body_handles) =
                serializer.finish();
            let mut world = self.components.world.borrow_mut();
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
                started: true,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                components: self.components,
            };
            let scene: (
                &Scene,
                Vec<Option<(&u32, &ComponentGroup)>>,
                FxHashMap<ComponentTypeId, Vec<(ComponentGroupId, Vec<Option<(u32, Vec<u8>)>>)>>,
                FxHashMap<StateTypeId, Vec<u8>>,
                FxHashMap<StateTypeId, Vec<u8>>,
            ) = (
                &scene,
                groups,
                ser_components,
                ser_scene_state,
                ser_global_state,
            );
            let result = bincode::serialize(&scene);

            *self.components.world.borrow_mut() = old_world;

            return result;
        }

        #[cfg(not(feature = "physics"))]
        {
            let (groups, ser_components, ser_scene_state, ser_global_state) = serializer.finish();
            let scene = Scene {
                id: *self.scene_id,
                resized: true,
                switched: true,
                started: true,
                screen_config: self.screen_config,
                world_camera: self.world_camera,
                components: self.components,
            };
            let scene: (
                &Scene,
                Vec<Option<(&u32, &ComponentGroup)>>,
                FxHashMap<ComponentTypeId, Vec<(ComponentGroupId, Vec<Option<(u32, Vec<u8>)>>)>>,
                FxHashMap<StateTypeId, Vec<u8>>,
                FxHashMap<StateTypeId, Vec<u8>>,
            ) = (
                &scene,
                groups,
                ser_components,
                ser_scene_state,
                ser_global_state,
            );
            let result = bincode::serialize(&scene);
            return result;
        }
    }

    pub fn remove_scene(&mut self, id: u32) -> Option<Scene> {
        if let Some(mut scene) = self.scenes.remove(id) {
            for end in scene.states.ends() {
                let mut ctx = Context::from_fields(ShuraFields::from_ctx(self), &mut scene);
                end(&mut ctx);
            }
            return Some(scene);
        }
        return None;
    }

    pub fn add_scene(&mut self, scene: impl SceneCreator) {
        let scene = scene.scene(ShuraFields::from_ctx(self));
        self.scenes.add(scene);
    }

    // pub fn render_each<C: ComponentController>(
    //     &'a self,
    //     encoder: &'a mut RenderEncoder,
    //     config: RenderConfig<'a>,
    //     mut each: impl FnMut(&mut Renderer<'a>, &'a C, InstanceIndex),
    // ) {
    //     let mut renderer = encoder.renderer(config);
    //     for (buffer, components) in self.components.iter_render::<C>(ComponentFilter::Active) {
    //         renderer.use_instances(buffer);
    //         for (instance, component) in components {
    //             (each)(&mut renderer, component, instance);
    //         }
    //     }
    // }

    // pub fn render_all<C: ComponentController>(
    //     &'a self,
    //     encoder: &'a mut RenderEncoder,
    //     config: RenderConfig<'a>,
    //     mut all: impl FnMut(&mut Renderer<'a>, InstanceIndices),
    // ) {
    //     let mut renderer = encoder.renderer(config);
    //     for (buffer, _) in self.components.iter_render::<C>(ComponentFilter::Active) {
    //         renderer.use_instances(buffer);
    //         (all)(&mut renderer, buffer.all_instances());
    //     }
    // }

    // #[cfg(feature = "physics")]
    // pub fn create_joint(
    //     &mut self,
    //     component1: &BaseComponent,
    //     component2: &BaseComponent,
    //     joint: impl Into<GenericJoint>,
    // ) -> ImpulseJointHandle {
    //     self.components
    //         .world_mut()
    //         .create_joint(component1, component2, joint)
    // }

    // pub fn scene_resized(&self) -> bool {
    //     *self.scene_resized
    // }

    // pub fn scene_switched(&self) -> bool {
    //     *self.scene_switched
    // }

    // pub fn scene_started(&self) -> bool {
    //     *self.scene_started
    // }

    // #[cfg(feature = "audio")]
    // pub fn create_sink(&self) -> Sink {
    //     Sink::try_new(&self.audio_handle).unwrap()
    // }

    // pub fn create_instance_buffer(&self, instances: &[Matrix]) -> InstanceBuffer {
    //     self.gpu.create_instance_buffer(instances)
    // }

    // pub fn create_model(&self, builder: ModelBuilder) -> Model {
    //     self.gpu.create_model(builder)
    // }

    // pub fn create_sprite(&self, bytes: &[u8]) -> Sprite {
    //     self.gpu.create_sprite(bytes)
    // }

    // pub fn create_camera_buffer(&self, camera: &Camera) -> CameraBuffer {
    //     self.gpu.create_camera_buffer(camera)
    // }

    // pub fn create_render_target(&self, size: Vector<u32>) -> RenderTarget {
    //     self.gpu.create_render_target(size)
    // }

    // pub fn create_sprite_from_image(&self, image: image::DynamicImage) -> Sprite {
    //     self.gpu.create_sprite_from_image(image)
    // }

    // pub fn create_empty_sprite(&self, size: Vector<u32>) -> Sprite {
    //     self.gpu.create_empty_sprite(size)
    // }

    // pub fn create_sprite_sheet(&self, bytes: &[u8], sprites: Vector<u32>) -> SpriteSheet {
    //     self.gpu.create_sprite_sheet(bytes, sprites)
    // }

    // #[cfg(feature = "text")]
    // pub fn create_font(&self, bytes: &'static [u8]) -> FontBrush {
    //     self.gpu.create_font(bytes)
    // }

    // #[cfg(feature = "text")]
    // pub fn create_text(
    //     &self,
    //     defaults: &GpuDefaults,
    //     target_size: Vector<u32>,
    //     descriptor: TextDescriptor,
    // ) -> RenderTarget {
    //     self.gpu.create_text(defaults, target_size, descriptor)
    // }

    // pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
    //     self.gpu.create_uniform(data)
    // }

    // pub fn create_shader(&self, config: ShaderConfig) -> Shader {
    //     self.gpu.create_shader(config)
    // }

    // pub fn create_computed_target<'caller>(
    //     &self,
    //     texture_size: Vector<u32>,
    //     camera: &CameraBuffer,
    //     compute: impl FnMut(RenderConfig, &mut RenderEncoder),
    // ) -> RenderTarget {
    //     self.gpu
    //         .create_computed_target(self.defaults, texture_size, camera, compute)
    // }

    // #[cfg(feature = "audio")]
    // pub fn create_sound(&self, sound: &'static [u8]) -> Sound {
    //     return Sound::new(sound);
    // }

    // #[cfg(feature = "animation")]
    // pub fn create_tween<T: Stepable>(
    //     &self,
    //     ease_function: impl Into<EaseMethod>,
    //     duration: Duration,
    //     start: T,
    //     end: T,
    // ) -> Tween<T> {
    //     return Tween::new(ease_function, duration, start, end);
    // }

    // #[cfg(feature = "animation")]
    // pub fn create_tween_sequence<T: Stepable>(
    //     &self,
    //     items: impl IntoIterator<Item = Tween<T>>,
    // ) -> TweenSequence<T> {
    //     return TweenSequence::new(items);
    // }

    // #[cfg(feature = "physics")]
    // pub fn create_collider<C: ComponentController>(
    //     &mut self,
    //     component: &C,
    //     collider: impl Into<Collider>,
    // ) -> ColliderHandle {
    //     let body_handle = component
    //         .base()
    //         .try_body_handle()
    //         .expect("Cannot add a collider to a component with no RigidBody!");
    //     let component_handle = component.base().handle();
    //     assert!(
    //         component_handle != ComponentHandle::INVALID,
    //         "Initialize the component before adding additional colliders!"
    //     );
    //     self.components.world_mut().create_collider(
    //         body_handle,
    //         component_handle,
    //         C::IDENTIFIER,
    //         collider,
    //     )
    // }

    // pub fn add_group(&mut self, group: impl Into<ComponentGroup>) {
    //     self.components.add_group(group);
    // }

    // pub fn add_component_to_group<C: ComponentController>(
    //     &mut self,
    //     group_id: ComponentGroupId,
    //     component: C,
    // ) -> ComponentHandle {
    //     return self
    //         .components
    //         .add_component_to_group(group_id, component);
    // }

    // pub fn add_component<C: ComponentController>(&mut self, component: C) -> ComponentHandle {
    //     return self.components.add_component(component);
    // }

    // pub fn add_components<I, C: ComponentController>(
    //     &mut self,
    //     components: I,
    // ) -> Vec<ComponentHandle>
    // where
    //     I: IntoIterator,
    //     I::IntoIter: ExactSizeIterator<Item = C>,
    // {
    //     return self.components.add_components(components);
    // }

    // pub fn add_components_to_group<I, C: ComponentController>(
    //     &mut self,
    //     group_id: ComponentGroupId,
    //     components: I,
    // ) -> Vec<ComponentHandle>
    // where
    //     I: IntoIterator,
    //     I::IntoIter: ExactSizeIterator<Item = C>,
    // {
    //     return self
    //         .components
    //         .add_components_to_group(group_id, components);
    // }

    // pub fn group_deltas(&self) -> &[GroupDelta] {
    //     self.components.group_deltas()
    // }

    // pub fn submit_encoders(&self) {
    //     self.gpu.submit_encoders()
    // }

    // pub fn remove_component(&mut self, handle: ComponentHandle) -> Option<BoxedComponent> {
    //     return self.components.remove_component(handle);
    // }

    // pub fn remove_components<C: ComponentController>(&mut self, filter: ComponentFilter) {
    //     self.components.remove_components::<C>(filter);
    // }

    // pub fn remove_group(&mut self, group_id: ComponentGroupId) -> Option<ComponentGroup> {
    //     self.components.remove_group(group_id)
    // }

    // pub fn try_global_state<T: GlobalStateController + StateIdentifier>(&self) -> Option<&T> {
    //     self.global_states.try_get::<T>()
    // }
    // pub fn try_global_state_mut<T: GlobalStateController + StateIdentifier>(
    //     &mut self,
    // ) -> Option<&mut T> {
    //     self.global_states.try_get_mut::<T>()
    // }
    // pub fn try_remove_global_state<T: GlobalStateController + StateIdentifier>(
    //     &mut self,
    // ) -> Option<Box<T>> {
    //     self.global_states.try_remove::<T>()
    // }
    // pub fn insert_global_state<T: GlobalStateController + StateIdentifier>(&mut self, state: T) {
    //     self.global_states.insert(state)
    // }
    // pub fn contains_global_state<T: GlobalStateController + StateIdentifier>(&self) -> bool {
    //     self.global_states.contains::<T>()
    // }
    // pub fn remove_global_state<T: GlobalStateController + StateIdentifier>(&mut self) -> Box<T> {
    //     self.global_states.remove::<T>()
    // }
    // pub fn global_state<T: GlobalStateController + StateIdentifier>(&self) -> &T {
    //     self.global_states.get::<T>()
    // }
    // pub fn global_state_mut<T: GlobalStateController + StateIdentifier>(&mut self) -> &mut T {
    //     self.global_states.get_mut::<T>()
    // }

    // pub fn try_scene_state<T: SceneStateController + StateIdentifier>(&self) -> Option<&T> {
    //     self.scene_states.try_get::<T>()
    // }
    // pub fn try_scene_state_mut<T: SceneStateController + StateIdentifier>(
    //     &mut self,
    // ) -> Option<&mut T> {
    //     self.scene_states.try_get_mut::<T>()
    // }
    // pub fn try_remove_scene_state<T: SceneStateController + StateIdentifier>(
    //     &mut self,
    // ) -> Option<Box<T>> {
    //     self.scene_states.try_remove::<T>()
    // }
    // pub fn insert_scene_state<T: SceneStateController + StateIdentifier>(&mut self, state: T) {
    //     self.scene_states.insert(state)
    // }
    // pub fn contains_scene_state<T: SceneStateController + StateIdentifier>(&self) -> bool {
    //     self.scene_states.contains::<T>()
    // }
    // pub fn remove_scene_state<T: SceneStateController + StateIdentifier>(&mut self) -> Box<T> {
    //     self.scene_states.remove::<T>()
    // }
    // pub fn scene_state<T: SceneStateController + StateIdentifier>(&self) -> &T {
    //     self.scene_states.get::<T>()
    // }
    // pub fn scene_state_mut<T: SceneStateController + StateIdentifier>(&mut self) -> &mut T {
    //     self.scene_states.get_mut::<T>()
    // }

    // pub fn render_scale(&self) -> f32 {
    //     self.screen_config.render_scale()
    // }

    // #[cfg(feature = "physics")]
    // pub fn remove_joint(&mut self, joint: ImpulseJointHandle) -> Option<ImpulseJoint> {
    //     self.components.world_mut().remove_joint(joint)
    // }

    // #[cfg(feature = "physics")]
    // pub fn remove_collider(&mut self, collider_handle: ColliderHandle) -> Option<Collider> {
    //     self.components
    //         .world_mut()
    //         .remove_collider(collider_handle)
    // }

    // #[cfg(feature = "physics")]
    // pub fn joint(
    //     &self,
    //     joint_handle: ImpulseJointHandle,
    // ) -> Option<impl Deref<Target = ImpulseJoint> + '_> {
    //     Ref::filter_map(self.components.world(), |w| w.joint(joint_handle)).ok()
    // }

    // #[cfg(feature = "physics")]
    // pub fn joint_mut(
    //     &mut self,
    //     joint_handle: ImpulseJointHandle,
    // ) -> Option<impl DerefMut<Target = ImpulseJoint> + '_> {
    //     RefMut::filter_map(self.components.world_mut(), |w| {
    //         w.joint_mut(joint_handle)
    //     })
    //     .ok()
    // }

    // #[cfg(feature = "physics")]
    // pub fn collider(
    //     &self,
    //     collider_handle: ColliderHandle,
    // ) -> Option<impl Deref<Target = Collider> + '_> {
    //     Ref::filter_map(self.components.world(), |w| {
    //         w.collider(collider_handle)
    //     })
    //     .ok()
    // }

    // #[cfg(feature = "physics")]
    // pub fn collider_mut(
    //     &mut self,
    //     collider_handle: ColliderHandle,
    // ) -> Option<impl DerefMut<Target = Collider> + '_> {
    //     RefMut::filter_map(self.components.world_mut(), |w| {
    //         w.collider_mut(collider_handle)
    //     })
    //     .ok()
    // }

    // #[cfg(feature = "physics")]
    // pub fn body(
    //     &self,
    //     body_handle: RigidBodyHandle,
    // ) -> Option<impl Deref<Target = RigidBody> + '_> {
    //     Ref::filter_map(self.components.world(), |w| w.body(body_handle)).ok()
    // }

    // #[cfg(feature = "physics")]
    // pub fn body_mut(
    //     &mut self,
    //     body_handle: RigidBodyHandle,
    // ) -> Option<impl DerefMut<Target = RigidBody> + '_> {
    //     RefMut::filter_map(self.components.world_mut(), |w| {
    //         w.body_mut(body_handle)
    //     })
    //     .ok()
    // }

    // #[cfg(feature = "physics")]
    // pub fn bodies(&self) -> impl Deref<Target = RigidBodySet> + '_ {
    //     Ref::map(self.components.world(), |w| w.bodies())
    // }

    // #[cfg(feature = "physics")]
    // pub fn colliders(&self) -> impl Deref<Target = ColliderSet> + '_ {
    //     Ref::map(self.components.world(), |w| w.colliders())
    // }

    // #[cfg(feature = "physics")]
    // pub fn world(&self) -> impl Deref<Target = World> + '_ {
    //     self.components.world()
    // }

    // #[cfg(feature = "physics")]
    // pub fn world_mut(&mut self) -> impl DerefMut<Target = World> + '_ {
    //     self.components.world_mut()
    // }

    // pub fn is_pressed(&self, trigger: impl Into<InputTrigger>) -> bool {
    //     self.input.is_pressed(trigger)
    // }

    // pub fn is_held(&self, trigger: impl Into<InputTrigger>) -> bool {
    //     self.input.is_held(trigger)
    // }

    // pub fn wheel_delta(&self) -> f32 {
    //     self.input.wheel_delta()
    // }

    // pub fn held_time(&self, trigger: impl Into<InputTrigger>) -> f32 {
    //     self.input.held_time(trigger)
    // }

    // pub fn held_time_duration(&self, trigger: impl Into<InputTrigger>) -> Option<Duration> {
    //     self.input.held_time_duration(trigger)
    // }

    // pub fn events(&self) -> impl Iterator<Item = (&InputTrigger, &InputEvent)> {
    //     self.input.events()
    // }

    // pub fn event(&self, trigger: impl Into<InputTrigger>) -> Option<&InputEvent> {
    //     self.input.event(trigger)
    // }

    // pub const fn modifiers(&self) -> Modifier {
    //     self.input.modifiers()
    // }

    // pub fn is_vsync(&self) -> bool {
    //     self.screen_config.vsync
    // }

    // pub fn render_size(&self) -> Vector<u32> {
    //     self.gpu.render_size(self.render_scale())
    // }

    // pub const fn total_frames(&self) -> u64 {
    //     self.frame.total_frames()
    // }

    // pub const fn start_time(&self) -> Instant {
    //     self.frame.start_time()
    // }

    // pub const fn update_time(&self) -> Instant {
    //     self.frame.update_time()
    // }

    // pub fn now(&self) -> Instant {
    //     self.frame.now()
    // }

    // pub fn render_components(&self) -> bool {
    //     self.components.render_components()
    // }

    // pub fn camera_fov(&self) -> Vector<f32> {
    //     self.world_camera.fov()
    // }

    // pub fn camera_translation(&self) -> &Vector<f32> {
    //     self.world_camera.translation()
    // }

    // pub fn camera_rotation(&self) -> &Rotation<f32> {
    //     self.world_camera.rotation()
    // }

    // pub fn camera_position(&self) -> &Isometry<f32> {
    //     self.world_camera.position()
    // }

    // pub fn camera_target(&self) -> Option<ComponentHandle> {
    //     self.world_camera.target()
    // }

    // pub fn clear_color(&self) -> Option<Color> {
    //     self.screen_config.clear_color()
    // }

    // pub fn cursor_camera(&self, camera: &Camera) -> Vector<f32> {
    //     let window_size = self.window_size();
    //     self.input.cursor_camera(window_size, camera)
    // }

    // pub fn cursor_world(&self) -> Vector<f32> {
    //     self.cursor_camera(&self.world_camera)
    // }

    // pub fn cursor_relative(&self) -> Vector<f32> {
    //     self.cursor_camera(&self.defaults.relative_camera.1)
    // }

    // pub fn cursor_relative_bottom_left(&self) -> Vector<f32> {
    //     self.cursor_camera(&self.defaults.relative_bottom_left_camera.1)
    // }

    // pub fn cursor_relative_bottom_right(&self) -> Vector<f32> {
    //     self.cursor_camera(&self.defaults.relative_bottom_right_camera.1)
    // }

    // pub fn cursor_relative_top_left(&self) -> Vector<f32> {
    //     self.cursor_camera(&self.defaults.relative_top_left_camera.1)
    // }

    // pub fn cursor_relative_top_right(&self) -> Vector<f32> {
    //     self.cursor_camera(&self.defaults.relative_top_right_camera.1)
    // }

    // // pub fn cursor_world(&self) -> Vector<f32> {
    // //     self.input.cursor_world()
    // // }

    // pub fn cursor_raw(&self) -> &Vector<u32> {
    //     self.input.cursor_raw()
    // }

    // pub fn touches_raw(&self) -> impl Iterator<Item = (&u64, &Vector<u32>)> {
    //     self.input.touches_raw()
    // }

    // pub fn window_size(&self) -> Vector<u32> {
    //     let mint: mint::Vector2<u32> = self.window.inner_size().into();
    //     return mint.into();
    // }

    // #[cfg(feature = "physics")]
    // pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
    //     self.components
    //         .world()
    //         .intersects_ray(collider_handle, ray, max_toi)
    // }

    // #[cfg(feature = "physics")]
    // pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Vector<f32>) -> bool {
    //     self.components
    //         .world()
    //         .intersects_point(collider_handle, point)
    // }

    // #[cfg(feature = "physics")]
    // pub fn test_filter(
    //     &self,
    //     filter: QueryFilter,
    //     handle: ColliderHandle,
    //     collider: &Collider,
    // ) -> bool {
    //     self.components
    //         .world()
    //         .test_filter(filter, handle, collider)
    // }

    // #[cfg(feature = "physics")]
    // pub fn cast_ray(
    //     &self,
    //     ray: &Ray,
    //     max_toi: f32,
    //     solid: bool,
    //     filter: QueryFilter,
    // ) -> Option<(ComponentHandle, ColliderHandle, f32)> {
    //     self.components
    //         .world()
    //         .cast_ray(ray, max_toi, solid, filter)
    // }

    // #[cfg(feature = "physics")]
    // pub fn cast_shape(
    //     &self,
    //     shape: &dyn Shape,
    //     position: &Isometry<f32>,
    //     velocity: &Vector<f32>,
    //     max_toi: f32,
    //     stop_at_penetration: bool,
    //     filter: QueryFilter,
    // ) -> Option<(ComponentHandle, ColliderHandle, TOI)> {
    //     self.components.world().cast_shape(
    //         shape,
    //         position,
    //         velocity,
    //         max_toi,
    //         stop_at_penetration,
    //         filter,
    //     )
    // }

    // #[cfg(feature = "physics")]
    // pub fn cast_ray_and_get_normal(
    //     &self,
    //     ray: &Ray,
    //     max_toi: f32,
    //     solid: bool,
    //     filter: QueryFilter,
    // ) -> Option<(ComponentHandle, ColliderHandle, RayIntersection)> {
    //     self.components
    //         .world()
    //         .cast_ray_and_get_normal(ray, max_toi, solid, filter)
    // }

    // #[cfg(feature = "physics")]
    // pub fn intersections_with_ray(
    //     &self,
    //     ray: &Ray,
    //     max_toi: f32,
    //     solid: bool,
    //     filter: QueryFilter,
    //     callback: impl FnMut(ComponentHandle, ColliderHandle, RayIntersection) -> bool,
    // ) {
    //     self.components
    //         .world()
    //         .intersections_with_ray(ray, max_toi, solid, filter, callback)
    // }

    // #[cfg(feature = "physics")]
    // pub fn intersections_with_shape(
    //     &self,
    //     shape_pos: &Isometry<f32>,
    //     shape: &dyn Shape,
    //     filter: QueryFilter,
    //     callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    // ) {
    //     self.components
    //         .world()
    //         .intersections_with_shape(shape_pos, shape, filter, callback)
    // }

    // #[cfg(feature = "physics")]
    // pub fn intersection_with_shape(
    //     &self,
    //     shape_pos: &Isometry<f32>,
    //     shape: &dyn Shape,
    //     filter: QueryFilter,
    // ) -> Option<(ComponentHandle, ColliderHandle)> {
    //     self.components
    //         .world()
    //         .intersection_with_shape(shape_pos, shape, filter)
    // }

    // #[cfg(feature = "physics")]
    // pub fn intersections_with_point(
    //     &self,
    //     point: &Point<f32>,
    //     filter: QueryFilter,
    //     callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    // ) {
    //     self.components
    //         .world()
    //         .intersections_with_point(point, filter, callback)
    // }

    // pub const fn total_time_duration(&self) -> Duration {
    //     self.frame.total_time_duration()
    // }

    // pub fn total_time(&self) -> f32 {
    //     self.frame.total_time()
    // }

    // pub const fn frame_time_duration(&self) -> Duration {
    //     self.frame.frame_time_duration()
    // }

    // pub fn frame_time(&self) -> f32 {
    //     self.frame.frame_time()
    // }

    // pub const fn fps(&self) -> u32 {
    //     self.frame.fps()
    // }

    // pub fn max_fps(&self) -> Option<u32> {
    //     self.screen_config.max_fps()
    // }

    // pub fn max_frame_time(&self) -> Option<Duration> {
    //     self.screen_config.max_frame_time()
    // }

    // pub fn window(&self) -> &winit::window::Window {
    //     &self.window
    // }

    // pub fn window_mut(&mut self) -> &mut winit::window::Window {
    //     &mut self.window
    // }

    // pub fn group_mut(&mut self, id: ComponentGroupId) -> Option<&mut ComponentGroup> {
    //     self.components.group_by_id_mut(id)
    // }

    // pub fn group(&self, id: ComponentGroupId) -> Option<&ComponentGroup> {
    //     self.components.group_by_id(id)
    // }

    // pub fn groups(&self) -> impl Iterator<Item = &ComponentGroup> {
    //     self.components.groups()
    // }

    // pub fn groups_mut(&mut self) -> impl Iterator<Item = &mut ComponentGroup> {
    //     self.components.groups_mut()
    // }

    // pub fn active_scene(&self) -> u32 {
    //     self.scenes.active_scene()
    // }

    // pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
    //     self.scenes.scene_ids()
    // }

    // pub fn does_scene_exist(&self, name: u32) -> bool {
    //     self.scenes.does_scene_exist(name)
    // }

    // pub fn active_group_ids(&self) -> &[ComponentGroupId] {
    //     self.components.active_group_ids()
    // }

    // pub fn group_ids(&self) -> impl Iterator<Item = &ComponentGroupId> {
    //     self.components.group_ids()
    // }

    // pub fn amount_of_components<C: ComponentController + ComponentDerive>(
    //     &self,
    //     group_id: ComponentGroupId,
    // ) -> usize {
    //     self.components.amount_of_components::<C>(group_id)
    // }

    // pub fn component_by_index<C: ComponentController + ComponentDerive>(
    //     &self,
    //     group_id: ComponentGroupId,
    //     index: u32,
    // ) -> Option<&C> {
    //     self.components
    //         .component_by_index::<C>(group_id, index)
    // }

    // pub fn component_by_index_mut<C: ComponentController + ComponentDerive>(
    //     &mut self,
    //     group_id: ComponentGroupId,
    //     index: u32,
    // ) -> Option<&mut C> {
    //     self.components
    //         .component_by_index_mut::<C>(group_id, index)
    // }

    // pub fn component<C: ComponentDerive>(&self, handle: ComponentHandle) -> Option<&C> {
    //     self.components.component::<C>(handle)
    // }

    // pub fn component_mut<C: ComponentDerive>(&mut self, handle: ComponentHandle) -> Option<&mut C> {
    //     self.components.component_mut::<C>(handle)
    // }

    // pub fn boxed_component(&self, handle: ComponentHandle) -> Option<&BoxedComponent> {
    //     self.components.boxed_component(handle)
    // }

    // pub fn boxed_component_mut(&mut self, handle: ComponentHandle) -> Option<&mut BoxedComponent> {
    //     self.components.boxed_component_mut(handle)
    // }

    // pub fn force_buffer<C: ComponentController>(&mut self, filter: ComponentFilter) {
    //     self.components.force_buffer::<C>(filter)
    // }

    // #[cfg(feature = "physics")]
    // pub fn gravity(&self) -> Vector<f32> {
    //     self.components.world().gravity()
    // }

    // #[cfg(feature = "physics")]
    // pub fn time_scale(&self) -> f32 {
    //     self.components.world().time_scale()
    // }

    // #[cfg(feature = "physics")]
    // pub fn physics_priority(&self) -> Option<i16> {
    //     self.components.world().physics_priority()
    // }

    // pub fn active_render<C: ComponentDerive>(
    //     &self,
    //     active: &ActiveComponents<C>,
    // ) -> ComponentRenderGroup<C> {
    //     return self.components.active_render(active, self.defaults);
    // }

    // pub fn active<C: ComponentDerive>(&self, active: &ActiveComponents<C>) -> ComponentSet<C> {
    //     return self.components.active(active);
    // }

    // pub fn active_mut<C: ComponentDerive>(
    //     &mut self,
    //     active: &ActiveComponents<C>,
    // ) -> ComponentSetMut<C> {
    //     return self.components.active_mut(active);
    // }

    // pub fn components_mut<C: ComponentController>(
    //     &mut self,
    //     filter: ComponentFilter,
    // ) -> ComponentSetMut<C> {
    //     self.components.components_mut::<C>(filter)
    // }

    // pub fn components<C: ComponentController>(&self, filter: ComponentFilter) -> ComponentSet<C> {
    //     self.components.components::<C>(filter)
    // }

    // pub fn instance_buffer<C: ComponentController>(
    //     &self,
    //     group_id: ComponentGroupId,
    // ) -> Option<&InstanceBuffer> {
    //     self.components.instance_buffer::<C>(group_id)
    // }

    // #[cfg(feature = "gamepad")]
    // pub fn gamepads(&self) -> Option<ConnectedGamepadsIterator> {
    //     self.input.gamepads()
    // }

    // #[cfg(feature = "gamepad")]
    // pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
    //     self.input.gamepad(gamepad_id)
    // }

    // pub fn set_render_scale(&mut self, scale: f32) {
    //     self.screen_config.set_render_scale(scale);
    // }

    // pub fn set_active_scene(&mut self, active_scene: u32) {
    //     self.scenes.set_active_scene(active_scene)
    // }

    // pub fn set_render_components(&mut self, render_components: bool) {
    //     self.components
    //         .set_render_components(render_components)
    // }

    // pub fn set_camera_position(&mut self, pos: Isometry<f32>) {
    //     self.world_camera.set_position(pos);
    // }

    // pub fn set_camera_translation(&mut self, translation: Vector<f32>) {
    //     self.world_camera.set_translation(translation);
    // }

    // pub fn set_camera_rotation(&mut self, rotation: Rotation<f32>) {
    //     self.world_camera.set_rotation(rotation);
    // }

    // pub fn set_camera_target(&mut self, target: Option<ComponentHandle>) {
    //     self.world_camera.set_target(target);
    // }

    // /// Tries to enable or disable vSync. The default is always vSync to be on.
    // /// So every device supports vSync but not every device supports no vSync.
    // pub fn set_vsync(&mut self, vsync: bool) {
    //     self.screen_config.set_vsync(vsync);
    // }

    // pub fn set_cursor_hidden(&mut self, hidden: bool) {
    //     self.window.set_cursor_visible(!hidden);
    // }

    // pub fn set_camera_scale(&mut self, scale: WorldCameraScale) {
    //     self.world_camera.set_fov_scale(scale, self.window_size())
    // }

    // pub fn set_window_size(&mut self, size: Vector<u32>) {
    //     let mint: mint::Vector2<u32> = size.into();
    //     let size: winit::dpi::PhysicalSize<u32> = mint.into();
    //     self.window.set_inner_size(size);
    // }

    // pub fn set_fullscreen(&mut self, fullscreen: bool) {
    //     if fullscreen {
    //         let f = winit::window::Fullscreen::Borderless(None);
    //         self.window.set_fullscreen(Some(f));
    //     } else {
    //         self.window.set_fullscreen(None);
    //     }
    // }

    // pub fn set_clear_color(&mut self, color: Option<Color>) {
    //     self.screen_config.set_clear_color(color);
    // }

    // pub fn set_window_resizable(&mut self, resizable: bool) {
    //     self.window.set_resizable(resizable);
    //     self.window
    //         .set_enabled_buttons(winit::window::WindowButtons::CLOSE)
    // }

    // pub fn set_window_title(&mut self, title: &str) {
    //     self.window.set_title(title);
    // }

    // #[cfg(feature = "physics")]
    // pub fn set_gravity(&mut self, gravity: Vector<f32>) {
    //     self.components.world_mut().set_gravity(gravity);
    // }

    // #[cfg(feature = "physics")]
    // pub fn set_time_scale(&mut self, time_scale: f32) {
    //     self.components
    //         .world_mut()
    //         .set_time_scale(time_scale);
    // }

    // #[cfg(feature = "physics")]
    // pub fn set_physics_priority(&mut self, step: Option<i16>) {
    //     self.components
    //         .world_mut()
    //         .set_physics_priority(step);
    // }

    // pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
    //     self.screen_config.set_max_fps(max_fps);
    // }

    // pub const fn frames_since_last_seconds(&self) -> u32 {
    //     self.frame.frames_since_last_seconds()
    // }
}
