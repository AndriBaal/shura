use crate::{
    data::arena::ArenaEntry, ArenaPath, Camera, Color, ComponentCluster, ComponentController,
    ComponentGroup, ComponentGroupDescriptor, ComponentHandle, ComponentManager, ComponentSet,
    ComponentSetMut, CursorManager, Dimension, DynamicComponent, DynamicScene, FrameManager, Gpu,
    Input, InputEvent, InputTrigger, InstanceBuffer, Instances, Isometry, Key, Matrix, Model,
    ModelBuilder, Modifier, Renderer, Rotation, Scene, SceneController, SceneManager, Shader,
    ShaderField, ShaderLang, ShuraCore, Sprite, SpriteSheet, Touch, Uniform, Vector,
};
use winit::window::Window;

#[cfg(feature = "audio")]
use crate::audio::{OutputStream, OutputStreamHandle, Sink, Sound};

#[cfg(feature = "physics")]
use crate::{physics::*, Point};
#[cfg(feature = "physics")]
use rapier2d::prelude::CollisionEvent;

#[cfg(feature = "gui")]
use crate::gui::{Gui, GuiContext};

#[cfg(feature = "text")]
use crate::text::{CreateFont, CreateText, Font, TextDescriptor};

#[cfg(feature = "gamepad")]
use crate::gamepad::*;

use instant::Duration;
use rustc_hash::FxHashMap;

macro_rules! Where {
    (
    $a:lifetime >= $b:lifetime $(,)?
) => {
        &$b & $a()
    };
}

pub(crate) struct RenderResources<'a> {
    #[cfg(feature = "gui")]
    pub gui: &'a mut Gui,
    #[cfg(feature = "gui")]
    pub window: &'a Window,
    pub clear_color: &'a Option<Color>,
    pub gpu: &'a Gpu,
    pub manager: &'a ComponentManager,
    pub camera: &'a Camera,
    pub saved_sprites: &'a mut Vec<(String, Sprite)>,
}

/// Context to communicate with the game engine to access components, scenes, camera, physics and many more.
pub struct Context<'a> {
    // Scene
    name: &'static str,
    pub camera: &'a mut Camera,
    pub component_manager: &'a mut ComponentManager,
    pub cursor: &'a mut CursorManager,
    #[cfg(feature = "physics")]
    pub world: &'a mut World,
    resized: &'a mut bool,
    switched: &'a mut bool,
    pub clear_color: &'a mut Option<Color>,
    pub saved_sprites: &'a mut Vec<(String, Sprite)>,

    // Core
    end: &'a mut bool,
    pub scene_manager: &'a mut SceneManager,
    pub frame_manager: &'a FrameManager,
    pub window: &'a mut Window,
    pub input: &'a mut Input,
    pub gpu: &'a mut Gpu,
    #[cfg(feature = "gui")]
    pub gui: &'a mut Gui,
    #[cfg(feature = "audio")]
    pub audio: &'a mut OutputStream,
    #[cfg(feature = "audio")]
    pub audio_handle: &'a mut OutputStreamHandle,
}

impl<'a> Context<'a> {
    pub(crate) fn new<S: SceneController, F: FnMut(&mut Context) -> S>(
        scene: &'a mut Scene,
        shura: &'a mut ShuraCore<S, F>,
    ) -> Context<'a> {
        let window = &mut shura.window;
        let input = &mut shura.input;

        if scene.resized {
            let new_size: Dimension<u32> = window.inner_size().into();
            scene
                .camera
                .resize(new_size.width as f32 / new_size.height as f32);
        }

        scene.cursor.compute(
            &scene.camera.fov(),
            &window.inner_size().into(),
            scene.camera.position(),
            input,
        );

        Self {
            name: scene.name,
            camera: &mut scene.camera,
            component_manager: &mut scene.component_manager,
            cursor: &mut scene.cursor,
            resized: &mut scene.resized,
            switched: &mut scene.switched,
            clear_color: &mut scene.clear_color,
            #[cfg(feature = "physics")]
            world: &mut scene.world,
            saved_sprites: &mut scene.saved_sprites,

            window,
            input,
            frame_manager: &mut shura.frame_manager,
            scene_manager: &mut shura.scene_manager,
            end: &mut shura.end,
            gpu: shura.gpu.as_mut().unwrap(),
            #[cfg(feature = "audio")]
            audio: &mut shura.audio.0,
            #[cfg(feature = "audio")]
            audio_handle: &mut shura.audio.1,
            #[cfg(feature = "gui")]
            gui: shura.gui.as_mut().unwrap(),
        }
    }

    pub(crate) fn new_manual(
        scene: &'a mut Scene,
        end: &'a mut bool,
        scene_manager: &'a mut SceneManager,
        frame_manager: &'a FrameManager,
        window: &'a mut Window,
        input: &'a mut Input,
        gpu: &'a mut Gpu,
        #[cfg(feature = "gui")] gui: &'a mut Gui,
        #[cfg(feature = "audio")] audio: &'a mut OutputStream,
        #[cfg(feature = "audio")] audio_handle: &'a mut OutputStreamHandle,
    ) -> Context<'a> {
        Self {
            name: scene.name,
            camera: &mut scene.camera,
            component_manager: &mut scene.component_manager,
            cursor: &mut scene.cursor,
            resized: &mut scene.resized,
            switched: &mut scene.switched,
            clear_color: &mut scene.clear_color,
            #[cfg(feature = "physics")]
            world: &mut scene.world,
            saved_sprites: &mut scene.saved_sprites,

            window,
            input,
            frame_manager,
            scene_manager,
            end,
            gpu,
            #[cfg(feature = "audio")]
            audio,
            #[cfg(feature = "audio")]
            audio_handle,
            #[cfg(feature = "gui")]
            gui,
        }
    }

    #[inline]
    pub(crate) fn copy_active_components(&self) -> Vec<ComponentCluster> {
        self.component_manager.copy_active_components()
    }

    #[inline]
    pub(crate) fn finish(self) -> RenderResources<'a> {
        *self.resized = false;
        *self.switched = false;
        return RenderResources {
            clear_color: self.clear_color,
            gpu: self.gpu,
            manager: self.component_manager,
            #[cfg(feature = "gui")]
            window: self.window,
            #[cfg(feature = "gui")]
            gui: self.gui,
            camera: self.camera,
            saved_sprites: self.saved_sprites,
        };
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub(crate) fn step_world(&mut self) {
        self.world.step(self.delta_time());
    }

    #[inline]
    pub(crate) fn remove_current_commponent(&mut self) -> bool {
        self.component_manager.remove_current_commponent()
    }

    #[inline]
    pub(crate) fn set_current_component(&mut self, current_component: Option<ComponentHandle>) {
        self.component_manager
            .set_current_component(current_component);
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub(crate) fn collision_event(
        &mut self,
    ) -> Result<CollisionEvent, crossbeam::channel::TryRecvError> {
        self.world.event_receivers.0.try_recv()
    }

    #[inline]
    pub(crate) fn normalize_input(&mut self) {
        self.input.update();
    }

    #[inline]
    pub(crate) fn update_sets(&mut self) {
        if let Some(target) = self.camera.target() {
            if let Some(component) = self.component_manager.component_dynamic(&target) {
                let matrix = component.inner().matrix(
                    #[cfg(feature = "physics")]
                    self.world,
                );
                self.camera
                    .set_translation(Vector::new(matrix[12], matrix[13]));
            } else {
                self.camera.set_target(None);
            }
        }

        self.component_manager.update_sets(&self.camera);
    }

    #[inline]
    pub(crate) fn buffer(&mut self) {
        self.camera.buffer(self.gpu);
        self.component_manager.buffer_sets(
            self.gpu,
            #[cfg(feature = "physics")]
            self.world,
        );
        self.gpu.update_defaults(
            self.frame_manager.total_time(),
            self.frame_manager.delta_time(),
        );
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn component_from_collider(&self, collider: &ColliderHandle) -> Option<ComponentHandle> {
        self.world.component(collider)
    }

    #[inline]
    pub fn does_group_exist(&self, group: u32) -> bool {
        self.component_manager.does_group_exist(group)
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Create
    //////////////////////////////////////////////////////////////////////////////////////////////

    #[inline]
    #[cfg(feature = "physics")]
    pub fn create_joint(
        &mut self,
        rigid_body1: RigidBodyHandle,
        rigid_body2: RigidBodyHandle,
        joint: impl Into<GenericJoint>,
    ) -> ImpulseJointHandle {
        self.world.create_joint(rigid_body1, rigid_body2, joint)
    }

    #[inline]
    #[cfg(feature = "audio")]
    pub fn create_sink(&self) -> Sink {
        Sink::try_new(&self.audio_handle).unwrap()
    }

    #[inline]
    pub fn create_instance_buffer(&self, instances: &[Matrix]) -> InstanceBuffer {
        InstanceBuffer::new(self.gpu, instances)
    }

    #[inline]
    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        Model::new(self.gpu, builder)
    }

    #[inline]
    pub fn create_sprite(&self, bytes: &[u8]) -> Sprite {
        Sprite::new(self.gpu, bytes)
    }

    #[inline]
    pub fn create_sprite_from_image(&self, image: image::DynamicImage) -> Sprite {
        Sprite::from_image(self.gpu, image)
    }

    #[inline]
    pub fn create_empty_sprite(&self, size: Dimension<u32>) -> Sprite {
        Sprite::empty(self.gpu, size)
    }

    #[inline]
    pub fn create_sprite_sheet(
        &self,
        bytes: &[u8],
        sprites: Dimension<u32>,
        sprite_size: Dimension<u32>,
    ) -> SpriteSheet {
        SpriteSheet::new(self.gpu, bytes, sprites, sprite_size)
    }

    #[inline]
    #[cfg(feature = "text")]
    pub fn create_font(&self, bytes: &'static [u8]) -> Font {
        Font::new_font(self.gpu, bytes)
    }

    #[inline]
    #[cfg(feature = "text")]
    pub fn create_text(&mut self, descriptor: TextDescriptor) -> Sprite {
        Sprite::new_text(self, descriptor)
    }

    #[inline]
    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        Uniform::new(self.gpu, data)
    }

    #[inline]
    pub fn create_shader(
        &self,
        code: &str,
        shader_type: ShaderLang,
        shader_fields: &[ShaderField],
    ) -> Shader {
        Shader::new(self.gpu, code, shader_type, shader_fields)
    }

    #[inline]
    pub fn create_custom_shader(&self, descriptor: &wgpu::RenderPipelineDescriptor) -> Shader {
        Shader::new_custom(self.gpu, descriptor)
    }

    #[inline]
    #[cfg(feature = "audio")]
    pub fn create_sound(&self, sound: &'static [u8]) -> Sound {
        return Sound::new(sound);
    }

    #[inline]
    pub fn create_computed_sprite<'caller, F>(
        &self,
        instances: &InstanceBuffer,
        fov: Dimension<f32>,
        camera: Isometry<f32>,
        texture_size: Dimension<u32>,
        clear_color: Option<Color>,
        compute: F,
    ) -> Sprite
    where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        return Sprite::computed(
            self.gpu,
            instances,
            fov,
            camera,
            texture_size,
            clear_color,
            compute,
        );
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn create_collider(
        &mut self,
        component: &PhysicsComponent,
        collider: &ColliderBuilder,
    ) -> ColliderHandle {
        self.world.create_collider(component, collider)
    }

    #[inline]
    pub fn create_group(&mut self, descriptor: &ComponentGroupDescriptor) {
        self.component_manager.create_group(descriptor);
    }

    #[inline]
    pub fn create_component<T: 'static + ComponentController>(
        &mut self,
        group: Option<u32>,
        controller: T,
    ) -> (&mut T, ComponentHandle) {
        self.component_manager.create_component(
            #[cfg(feature = "physics")]
            self.world,
            self.frame_manager.total_frames(),
            group,
            controller,
        )
    }

    #[inline]
    pub fn create_scene<S: SceneController, F: 'static + FnMut(&mut Context) -> S>(
        &mut self,
        name: &'static str,
        mut controller: F,
    ) {
        let window_size: Dimension<f32> = self.window_size().into();
        let ratio = window_size.width / window_size.height;
        let mut scene = Scene::new(self.gpu, ratio, name);
        let mut ctx = Context::new_manual(
            &mut scene,
            self.end,
            self.scene_manager,
            self.frame_manager,
            self.window,
            self.input,
            self.gpu,
            #[cfg(feature = "gui")]
            self.gui,
            #[cfg(feature = "audio")]
            self.audio,
            #[cfg(feature = "audio")]
            self.audio_handle,
        );
        let controller: DynamicScene = Box::new(controller(&mut ctx));
        self.scene_manager.add((controller, scene));
    }

    /// Remove a scene by its name
    #[inline]
    pub fn remove_scene(&mut self, name: &'static str) {
        if let Some((mut controller, mut scene)) = self.scene_manager.remove(name) {
            let mut ctx = Context::new_manual(
                &mut scene,
                self.end,
                self.scene_manager,
                self.frame_manager,
                self.window,
                self.input,
                self.gpu,
                #[cfg(feature = "gui")]
                self.gui,
                #[cfg(feature = "audio")]
                self.audio,
                #[cfg(feature = "audio")]
                self.audio_handle,
            );
            controller.end(&mut ctx);
        }
    }

    #[inline]
    pub fn remove_component(&mut self, handle: &ComponentHandle) -> Option<DynamicComponent> {
        return self.component_manager.remove_component(
            handle,
            #[cfg(feature = "physics")]
            self.world,
        );
    }

    #[inline]
    pub fn remove_components<T: ComponentController>(&mut self, groups: Option<&[u32]>) {
        self.component_manager.remove_components::<T>(
            groups,
            #[cfg(feature = "physics")]
            self.world,
        );
    }

    #[inline]
    pub fn remove_group(&mut self, group_id: u32) {
        self.component_manager.remove_group(
            group_id,
            #[cfg(feature = "physics")]
            self.world,
        )
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn remove_joint(&mut self, joint: ImpulseJointHandle) {
        self.world.remove_joint(joint)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn remove_collider(&mut self, collider_handle: ColliderHandle) {
        self.world.remove_collider(collider_handle);
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Getter
    //////////////////////////////////////////////////////////////////////////////////////////////
    #[inline]
    pub fn saved_sprites(&self) -> &Vec<(String, Sprite)> {
        &self.saved_sprites
    }

    #[inline]
    pub fn saved_sprites_mut(&mut self) -> &mut Vec<(String, Sprite)> {
        &mut self.saved_sprites
    }

    #[inline]
    pub fn clear_saved_sprites(&mut self) -> Vec<(String, Sprite)> {
        return std::mem::replace(&mut self.saved_sprites, vec![]);
    }

    #[inline]
    pub fn render_scale(&self) -> f32 {
        self.gpu.render_scale()
    }

    #[inline]
    #[cfg(feature = "gui")]
    pub fn gui(&self) -> GuiContext {
        self.gui.context()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.world.joint(joint)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.world.joint_mut(joint)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn collider(&self, collider_handle: ColliderHandle) -> Option<&Collider> {
        self.world.collider(collider_handle)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn collider_mut(&mut self, collider_handle: ColliderHandle) -> Option<&mut Collider> {
        self.world.collider_mut(collider_handle)
    }

    #[inline]
    pub fn is_pressed(&self, trigger: impl Into<InputTrigger>) -> bool {
        self.input.is_pressed(trigger)
    }

    #[inline]
    pub fn is_held(&self, trigger: impl Into<InputTrigger>) -> bool {
        self.input.is_held(trigger)
    }

    #[inline]
    pub fn wheel_delta(&self) -> f32 {
        self.input.wheel_delta()
    }

    #[inline]
    pub fn held_time(&self, trigger: impl Into<InputTrigger>) -> f32 {
        self.input.held_time(trigger)
    }

    #[inline]
    pub fn held_time_duration(&self, trigger: impl Into<InputTrigger>) -> Option<Duration> {
        self.input.held_time_duration(trigger)
    }

    #[inline]
    pub fn triggers(&self) -> &FxHashMap<InputTrigger, InputEvent> {
        self.input.triggers()
    }

    #[inline]
    pub fn staged_key(&self) -> Option<Key> {
        self.input.staged_key()
    }

    #[inline]
    pub const fn modifiers(&self) -> Option<Modifier> {
        self.input.modifiers()
    }

    #[inline]
    pub fn is_vsync(&self) -> bool {
        self.gpu.is_vsync()
    }

    #[inline]
    pub fn render_size(&self) -> Dimension<u32> {
        self.gpu.render_size()
    }

    #[inline]
    pub const fn total_frames(&self) -> u64 {
        self.frame_manager.total_frames()
    }

    #[inline]
    pub const fn render_components(&self) -> bool {
        self.component_manager.render_components()
    }

    #[inline]
    pub fn update_components(&self) -> bool {
        self.component_manager.update_components()
    }

    #[inline]
    /// Returns a dimension with the distance from the center of the camera to the right and from the
    /// center to the top.
    pub fn camera_fov(&self) -> Dimension<f32> {
        self.camera.fov()
    }

    #[inline]
    pub fn camera_translation(&self) -> &Vector<f32> {
        self.camera.translation()
    }

    #[inline]
    pub fn camera_rotation(&self) -> &Rotation<f32> {
        self.camera.rotation()
    }

    #[inline]
    pub fn camera_position(&self) -> &Isometry<f32> {
        self.camera.position()
    }

    #[inline]
    pub fn camera_target(&self) -> Option<ComponentHandle> {
        self.camera.target()
    }

    #[inline]
    pub fn clear_color(&self) -> Option<Color> {
        *self.clear_color
    }

    #[inline]
    pub fn cursor_world(&self) -> &Vector<f32> {
        self.cursor.cursor_world()
    }

    #[inline]
    pub fn relative_cursor_pos(&self) -> &Vector<f32> {
        self.cursor.cursor_relative()
    }

    #[inline]
    pub fn cursor_raw(&self) -> &Vector<f32> {
        self.input.cursor_raw()
    }

    #[inline]
    pub fn touches(&self) -> &[Touch] {
        self.cursor.touches()
    }

    #[inline]
    pub fn resized(&self) -> bool {
        return *self.resized;
    }

    #[inline]
    pub fn scene_switched(&self) -> bool {
        return *self.switched;
    }

    #[inline]
    pub fn end(&mut self, end: bool) {
        *self.end = end
    }

    #[inline]
    pub fn window_size(&self) -> Dimension<u32> {
        self.window.inner_size().into()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
        self.world.intersects_ray(collider_handle, ray, max_toi)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Vector<f32>) -> bool {
        self.world.intersects_point(collider_handle, point)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn test_filter(
        &mut self,
        filter: QueryFilter,
        handle: ColliderHandle,
        collider: &Collider,
    ) -> bool {
        self.world.test_filter(filter, handle, collider)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, f32)> {
        self.world.cast_ray(ray, max_toi, solid, filter)
    }

    #[inline]
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
        self.world.cast_shape(
            shape,
            position,
            velocity,
            max_toi,
            stop_at_penetration,
            filter,
        )
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn cast_ray_and_get_normal(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, RayIntersection)> {
        self.world
            .cast_ray_and_get_normal(ray, max_toi, solid, filter)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersections_with_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle, RayIntersection) -> bool,
    ) {
        self.world
            .intersections_with_ray(ray, max_toi, solid, filter, callback)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersections_with_shape(
        &self,
        shape_pos: &Isometry<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.world
            .intersections_with_shape(shape_pos, shape, filter, callback)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersection_with_shape(
        &self,
        shape_pos: &Isometry<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle)> {
        self.world.intersection_with_shape(shape_pos, shape, filter)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersections_with_point(
        &self,
        point: &Point<f32>,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.world.intersections_with_point(point, filter, callback)
    }

    #[inline]
    pub fn total_time_duration(&self) -> Duration {
        self.frame_manager.total_time_duration()
    }

    #[inline]
    pub fn delta_time_duration(&self) -> Duration {
        self.frame_manager.delta_time_duration()
    }

    #[inline]
    pub fn total_time(&self) -> f32 {
        self.frame_manager.total_time()
    }

    #[inline]
    pub fn delta_time(&self) -> f32 {
        self.frame_manager.delta_time()
    }

    #[inline]
    pub fn fps(&self) -> u32 {
        self.frame_manager.fps()
    }

    #[inline]
    pub fn window_mut(&mut self) -> &mut winit::window::Window {
        &mut self.window
    }

    #[inline]
    pub fn group_mut(&mut self, id: u32) -> Option<&mut ComponentGroup> {
        if let Some(group_index) = self.component_manager.group_index(&id) {
            return self.component_manager.group_mut(*group_index);
        }
        return None;
    }

    #[inline]
    pub fn group(&self, id: u32) -> Option<&ComponentGroup> {
        if let Some(group_index) = self.component_manager.group_index(&id) {
            return self.component_manager.group(*group_index);
        }
        return None;
    }

    #[inline]
    pub fn scene_name(&self) -> &str {
        self.name
    }

    #[inline]
    pub fn active_scene(&self) -> &'static str {
        self.scene_manager.active_scene()
    }

    #[inline]
    pub fn scenes(&self) -> Vec<&'static str> {
        self.scene_manager.scenes()
    }

    #[inline]
    pub fn does_scene_exist(&self, name: &'static str) -> bool {
        self.scene_manager.does_scene_exist(name)
    }

    #[inline]
    pub fn active_group_ids(&self) -> &[u32] {
        self.component_manager.active_group_ids()
    }

    #[inline]
    pub fn group_ids(&self) -> Vec<u32> {
        self.component_manager.group_ids()
    }

    #[inline]
    pub fn component_dynamic(&self, handle: &ComponentHandle) -> Option<&DynamicComponent> {
        self.component_manager.component_dynamic(handle)
    }

    pub fn component_dynamic_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut DynamicComponent> {
        self.component_manager.component_dynamic_mut(handle)
    }

    #[inline]
    pub fn component<T: ComponentController>(&self, handle: &ComponentHandle) -> Option<&T> {
        self.component_manager.component::<T>(handle)
    }

    #[inline]
    pub fn component_mut<T: ComponentController>(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut T> {
        self.component_manager.component_mut::<T>(handle)
    }

    #[inline]
    /// Force the position of the all component from the given generic to be updated inside the
    /// (InstanceBuffer)[crate::InstanceBuffer]. This is used when the [crate::ComponentConfig::does_move]
    /// flag is set, but the position needs to be updated.
    pub fn force_buffer<T: ComponentController>(&mut self) {
        self.component_manager.force_buffer::<T>()
    }

    #[inline]
    /// Force the position of the components from the given groups from the given generic to be updated inside the
    /// (InstanceBuffer)[crate::InstanceBuffer]. This is used when the [crate::ComponentConfig::does_move]
    /// flag is set, but the position needs to be updated.
    pub fn force_buffer_groups<T: ComponentController>(&mut self, group_ids: &[u32]) {
        self.component_manager.force_buffer_groups::<T>(group_ids)
    }

    #[inline]
    /// Force the position of the active components from the given generic to be updated inside the
    /// (InstanceBuffer)[crate::InstanceBuffer]. This is used when the [crate::ComponentConfig::does_move]
    /// flag is set, but the position needs to be updated.
    pub fn force_buffer_active<T: ComponentController>(&mut self) {
        self.component_manager.force_buffer_active::<T>()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn gravity(&self) -> &Vector<f32> {
        self.world.gravity()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn physics_priority(&self) -> i16 {
        self.world.physics_priority()
    }

    #[inline]
    pub fn components_mut<T: 'static + ComponentController>(
        &mut self,
        groups: Option<&[u32]>,
    ) -> ComponentSetMut<T> {
        self.component_manager.components_mut::<T>(groups)
    }

    #[inline]
    pub fn components<T: 'static + ComponentController>(
        &self,
        groups: Option<&[u32]>,
    ) -> ComponentSet<T> {
        self.component_manager.components::<T>(groups)
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn gamepads(&self) -> Option<ConnectedGamepadsIterator> {
        self.input.gamepads()
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
        self.input.gamepad(gamepad_id)
    }

    #[inline]
    pub(crate) fn borrow_component(
        &mut self,
        path: ArenaPath,
        index: usize,
    ) -> Option<ArenaEntry<DynamicComponent>> {
        self.component_manager.borrow_component(path, index)
    }

    #[inline]
    pub(crate) fn return_component(
        &mut self,
        path: ArenaPath,
        index: usize,
        component: ArenaEntry<DynamicComponent>,
    ) {
        self.component_manager
            .return_component(path, index, component)
    }

    #[inline]
    pub(crate) fn not_return_component(&mut self, path: ArenaPath, index: usize) {
        self.component_manager.not_return_component(path, index)
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Setter
    //////////////////////////////////////////////////////////////////////////////////////////////

    #[inline]
    pub fn set_render_scale(&mut self, scale: f32) {
        self.gpu.set_render_scale(scale)
    }

    #[inline]
    pub fn set_active_scene(&mut self, active_scene: &'static str) {
        self.scene_manager.set_active_scene(active_scene)
    }

    #[inline]
    pub fn set_update_components(&mut self, update_components: bool) {
        self.component_manager
            .set_update_components(update_components)
    }

    #[inline]
    pub fn set_render_components(&mut self, render_components: bool) {
        self.component_manager
            .set_render_components(render_components)
    }

    #[inline]
    pub fn set_camera_position(&mut self, pos: Isometry<f32>) {
        self.camera.set_position(pos);
    }

    #[inline]
    pub fn set_camera_translation(&mut self, translation: Vector<f32>) {
        self.camera.set_translation(translation);
    }

    #[inline]
    pub fn set_camera_rotation(&mut self, rotation: Rotation<f32>) {
        self.camera.set_rotation(rotation);
    }

    pub fn set_camera_target(&mut self, target: Option<ComponentHandle>) {
        self.camera.set_target(target);
    }

    #[inline]
    /// Tries to enable or disable vSync. The default is always vSync to be on.
    /// So every device supports vSync but not every device supports no vSync.
    pub fn set_vsync(&mut self, vsync: bool) {
        self.gpu.set_vsync(vsync);
    }

    #[inline]
    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.window.set_cursor_visible(!hidden);
    }

    #[inline]
    /// Set the distance between the center of the camera to the top in world coordinates.
    pub fn set_vertical_fov(&mut self, fov: f32) {
        self.camera.set_vertical_fov(fov);
    }

    #[inline]
    /// Set the distance between the center of the camera to the right in world coordinates.
    pub fn set_horizontal_fov(&mut self, fov: f32) {
        self.camera.set_horizontal_fov(fov);
    }

    #[inline]
    pub fn set_window_size(&mut self, size: Dimension<u32>) {
        let size: winit::dpi::PhysicalSize<u32> = size.into();
        self.window.set_inner_size(size);
    }

    #[inline]
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if fullscreen {
            let f = winit::window::Fullscreen::Borderless(None);
            self.window.set_fullscreen(Some(f));
        } else {
            self.window.set_fullscreen(None);
        }
    }

    #[inline]
    pub fn set_clear_color(&mut self, color: Option<Color>) {
        *self.clear_color = color;
    }

    #[inline]
    pub fn set_window_resizable(&mut self, resizable: bool) {
        self.window.set_resizable(resizable);
    }

    #[inline]
    pub fn set_window_title(&mut self, title: &str) {
        self.window.set_title(title);
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.world.set_gravity(gravity);
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn set_physics_priority(&mut self, step: i16) {
        self.world.set_physics_priority(step);
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn set_mapping(
        &mut self,
        gamepad_id: GamepadId,
        mapping: &Mapping,
        name: Option<&str>,
    ) -> Result<String, MappingError> {
        self.input.set_mapping(gamepad_id, mapping, name)
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn set_mapping_strict(
        &mut self,
        gamepad_id: GamepadId,
        mapping: &Mapping,
        name: Option<&str>,
    ) -> Result<String, MappingError> {
        self.input.set_mapping_strict(gamepad_id, mapping, name)
    }
}
