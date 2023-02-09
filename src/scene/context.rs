use crate::{
    data::arena::ArenaEntry, ArenaPath, CameraBuffers, Color, ComponentCluster,
    ComponentController, ComponentGroup, ComponentGroupDescriptor, ComponentHandle,
    ComponentIdentifier, ComponentSet, ComponentSetMut, Dimension, DynamicComponent, GroupFilter,
    InputEvent, InputTrigger, InstanceBuffer, Instances, Isometry, Key, Matrix, Model,
    ModelBuilder, Modifier, Renderer, Rotation, Scene, Shader, ShaderField, ShaderLang, Shura,
    Sprite, SpriteSheet, Touch, Uniform, Vector, ComponentTypeId
};

#[cfg(feature = "serialize")]
use crate::SceneSerializer;

#[cfg(feature = "audio")]
use crate::audio::{Sink, Sound};

#[cfg(feature = "physics")]
use crate::{physics::*, Point};

#[cfg(feature = "gui")]
use crate::gui::GuiContext;

#[cfg(feature = "text")]
use crate::text::{CreateFont, CreateText, Font, TextDescriptor};

#[cfg(feature = "gamepad")]
use crate::gamepad::*;

use instant::{Duration, Instant};
use rustc_hash::FxHashMap;

macro_rules! Where {
    (
    $a:lifetime >= $b:lifetime $(,)?
) => {
        &$b & $a()
    };
}

/// Context to communicate with the game engine to access components, scenes, camera, physics and many more.
pub struct Context<'a> {
    // Scene
    pub scene: &'a mut Scene,
    // Core
    pub shura: &'a mut Shura,
}

impl<'a> Context<'a> {
    #[inline]
    #[cfg(feature = "physics")]
    pub fn component_from_collider(&self, collider: &ColliderHandle) -> Option<(ComponentTypeId, ComponentHandle)> {
        self.scene.world.component(collider)
    }

    #[inline]
    pub fn does_group_exist(&self, group: u32) -> bool {
        self.scene.component_manager.does_group_exist(group)
    }

    #[cfg(feature = "serialize")]
    pub fn serialize(
        &'a mut self,
        mut serialize: impl FnMut(&mut SceneSerializer),
        pretty: bool,
    ) -> Option<String> {
        let mut s = SceneSerializer::new(
            self.scene,
            self.scene.component_manager.current_type(),
        );
        (serialize)(&mut s);
        return s.serialize(pretty);
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
        self.scene
            .world
            .create_joint(rigid_body1, rigid_body2, joint)
    }

    #[inline]
    #[cfg(feature = "audio")]
    pub fn create_sink(&self) -> Sink {
        Sink::try_new(&self.shura.audio_handle).unwrap()
    }

    #[inline]
    pub fn create_instance_buffer(&self, instances: &[Matrix]) -> InstanceBuffer {
        InstanceBuffer::new(&self.shura.gpu, instances)
    }

    #[inline]
    pub fn create_model(&self, builder: ModelBuilder) -> Model {
        Model::new(&self.shura.gpu, builder)
    }

    #[inline]
    pub fn create_sprite(&self, bytes: &[u8]) -> Sprite {
        Sprite::new(&self.shura.gpu, bytes)
    }

    #[inline]
    pub fn create_sprite_from_image(&self, image: image::DynamicImage) -> Sprite {
        Sprite::from_image(&self.shura.gpu, image)
    }

    #[inline]
    pub fn create_empty_sprite(&self, size: Dimension<u32>) -> Sprite {
        Sprite::empty(&self.shura.gpu, size)
    }

    #[inline]
    pub fn create_sprite_sheet(
        &self,
        bytes: &[u8],
        sprites: Dimension<u32>,
        sprite_size: Dimension<u32>,
    ) -> SpriteSheet {
        SpriteSheet::new(&self.shura.gpu, bytes, sprites, sprite_size)
    }

    #[inline]
    #[cfg(feature = "text")]
    pub fn create_font(&self, bytes: &'static [u8]) -> Font {
        Font::new_simple(&self.shura.gpu, bytes)
    }

    #[inline]
    #[cfg(feature = "text")]
    pub fn create_text(&mut self, descriptor: TextDescriptor) -> Sprite {
        Sprite::new_text(&self.shura.gpu, &mut self.shura.defaults, descriptor)
    }

    #[inline]
    pub fn create_uniform<T: bytemuck::Pod>(&self, data: T) -> Uniform<T> {
        Uniform::new(&self.shura.gpu, data)
    }

    #[inline]
    pub fn create_shader(
        &self,
        code: &str,
        shader_type: ShaderLang,
        shader_fields: &[ShaderField],
    ) -> Shader {
        Shader::new(&self.shura.gpu, code, shader_type, shader_fields)
    }

    #[inline]
    pub fn create_custom_shader(
        &self,
        shader_lang: ShaderLang,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> Shader {
        Shader::new_custom(&self.shura.gpu, shader_lang, descriptor)
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
        camera: &CameraBuffers,
        texture_size: Dimension<u32>,
        clear_color: Option<Color>,
        compute: F,
    ) -> Sprite
    where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        return Sprite::computed(
            &self.shura.gpu,
            &self.shura.defaults,
            instances,
            camera,
            texture_size,
            clear_color,
            compute,
        );
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn create_collider<C: ComponentController + ComponentIdentifier>(
        &mut self,
        component: &C,
        collider: &ColliderBuilder,
    ) -> ColliderHandle {
        self.scene.world.create_collider(component, collider)
    }

    #[inline]
    pub fn create_group(&mut self, descriptor: &ComponentGroupDescriptor) {
        self.scene.component_manager.create_group(descriptor);
    }

    #[inline]
    pub fn create_component<C: ComponentController + ComponentIdentifier>(
        &mut self,
        group: Option<u32>,
        component: C,
    ) -> (&mut C, ComponentHandle) {
        self.scene.component_manager.create_component(
            #[cfg(feature = "physics")]
            &mut self.scene.world,
            self.shura.frame_manager.total_frames(),
            group,
            component,
        )
    }

    #[inline]
    pub fn create_scene(&mut self, scene: Scene) {
        self.shura.scene_manager.add(scene);
    }

    /// Remove a scene by its name
    #[inline]
    pub fn remove_scene(&mut self, name: u32) -> Option<Scene> {
        if let Some(scene) = self.shura.scene_manager.remove(name) {
            return Some(scene);
        }
        return None;
    }

    #[inline]
    pub fn remove_component(&mut self, handle: &ComponentHandle) -> Option<DynamicComponent> {
        return self.scene.component_manager.remove_component(
            handle,
            #[cfg(feature = "physics")]
            &mut self.scene.world,
        );
    }

    #[inline]
    pub fn remove_components<C: ComponentController + ComponentIdentifier>(
        &mut self,
        filter: GroupFilter,
    ) {
        self.scene.component_manager.remove_components::<C>(
            filter,
            #[cfg(feature = "physics")]
            &mut self.scene.world,
        );
    }

    #[inline]
    pub fn remove_group(&mut self, group_id: u32) {
        self.scene.component_manager.remove_group(
            group_id,
            #[cfg(feature = "physics")]
            &mut self.scene.world,
        )
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn remove_joint(&mut self, joint: ImpulseJointHandle) {
        self.scene.world.remove_joint(joint)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn remove_collider(&mut self, collider_handle: ColliderHandle) {
        self.scene.world.remove_collider(collider_handle);
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Getter
    //////////////////////////////////////////////////////////////////////////////////////////////
    #[inline]
    pub fn current_component(&self) -> ComponentHandle {
        self.scene.component_manager.current_component()
    }

    #[inline]
    pub fn relative_camera(&self) -> &CameraBuffers {
        &self.shura.defaults.relative_camera
    }

    #[inline]
    pub fn saved_sprites(&self) -> &Vec<(String, Sprite)> {
        &self.scene.saved_sprites
    }

    #[inline]
    pub fn saved_sprites_mut(&mut self) -> &mut Vec<(String, Sprite)> {
        &mut self.scene.saved_sprites
    }

    #[inline]
    pub fn clear_saved_sprites(&mut self) -> Vec<(String, Sprite)> {
        return std::mem::replace(&mut self.scene.saved_sprites, vec![]);
    }

    #[inline]
    pub fn render_scale(&self) -> f32 {
        self.scene.render_config.render_scale()
    }

    #[inline]
    #[cfg(feature = "gui")]
    pub fn gui(&self) -> GuiContext {
        self.shura.gui.context()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.scene.world.joint(joint)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.scene.world.joint_mut(joint)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn collider(&self, collider_handle: ColliderHandle) -> Option<&Collider> {
        self.scene.world.collider(collider_handle)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn collider_mut(&mut self, collider_handle: ColliderHandle) -> Option<&mut Collider> {
        self.scene.world.collider_mut(collider_handle)
    }

    #[inline]
    pub fn is_pressed(&self, trigger: impl Into<InputTrigger>) -> bool {
        self.shura.input.is_pressed(trigger)
    }

    #[inline]
    pub fn is_held(&self, trigger: impl Into<InputTrigger>) -> bool {
        self.shura.input.is_held(trigger)
    }

    #[inline]
    pub fn wheel_delta(&self) -> f32 {
        self.shura.input.wheel_delta()
    }

    #[inline]
    pub fn held_time(&self, trigger: impl Into<InputTrigger>) -> f32 {
        self.shura.input.held_time(trigger)
    }

    #[inline]
    pub fn held_time_duration(&self, trigger: impl Into<InputTrigger>) -> Option<Duration> {
        self.shura.input.held_time_duration(trigger)
    }

    #[inline]
    pub fn triggers(&self) -> &FxHashMap<InputTrigger, InputEvent> {
        self.shura.input.triggers()
    }

    #[inline]
    pub fn staged_keys(&self) -> &[Key] {
        self.shura.input.staged_keys()
    }

    #[inline]
    pub const fn modifiers(&self) -> Modifier {
        self.shura.input.modifiers()
    }

    #[inline]
    pub fn is_vsync(&self) -> bool {
        self.shura.gpu.is_vsync()
    }

    #[inline]
    pub fn render_size(&self) -> Dimension<u32> {
        self.shura.gpu.render_size(self.render_scale())
    }

    #[inline]
    pub const fn total_frames(&self) -> u64 {
        self.shura.frame_manager.total_frames()
    }

    #[inline]
    pub const fn start_time(&self) -> Instant {
        self.shura.frame_manager.start_time()
    }

    #[inline]
    pub const fn update_time(&self) -> Instant {
        self.shura.frame_manager.update_time()
    }

    #[inline]
    pub fn now(&self) -> Instant {
        self.shura.frame_manager.now()
    }

    #[inline]
    pub fn render_components(&self) -> bool {
        self.scene.component_manager.render_components()
    }

    #[inline]
    pub fn update_components(&self) -> bool {
        self.scene.component_manager.update_components()
    }

    #[inline]
    /// Returns a dimension with the distance from the center of the camera to the right and from the
    /// center to the top.
    pub fn camera_fov(&self) -> Dimension<f32> {
        self.scene.camera.fov()
    }

    #[inline]
    pub fn camera_translation(&self) -> &Vector<f32> {
        self.scene.camera.translation()
    }

    #[inline]
    pub fn camera_rotation(&self) -> &Rotation<f32> {
        self.scene.camera.rotation()
    }

    #[inline]
    pub fn camera_position(&self) -> &Isometry<f32> {
        self.scene.camera.position()
    }

    #[inline]
    pub fn camera_target(&self) -> Option<ComponentHandle> {
        self.scene.camera.target()
    }

    #[inline]
    pub fn clear_color(&self) -> Option<Color> {
        self.scene.render_config.clear_color()
    }

    #[inline]
    pub fn cursor_world(&self) -> &Vector<f32> {
        self.scene.cursor.cursor_world()
    }

    #[inline]
    pub fn relative_cursor_pos(&self) -> &Vector<f32> {
        self.scene.cursor.cursor_relative()
    }

    #[inline]
    pub fn cursor_raw(&self) -> &Vector<u32> {
        self.scene.cursor.cursor_raw()
    }

    #[inline]
    pub fn touches(&self) -> &[Touch] {
        self.scene.cursor.touches()
    }

    #[inline]
    pub fn scene_resized(&self) -> bool {
        return self.scene.resized;
    }

    #[inline]
    pub fn scene_switched(&self) -> bool {
        return self.scene.switched;
    }

    #[inline]
    pub fn end(&mut self, end: bool) {
        self.shura.end = end
    }

    #[inline]
    pub fn window_size(&self) -> Dimension<u32> {
        self.shura.window.inner_size().into()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
        self.scene
            .world
            .intersects_ray(collider_handle, ray, max_toi)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Vector<f32>) -> bool {
        self.scene.world.intersects_point(collider_handle, point)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn test_filter(
        &self,
        filter: QueryFilter,
        handle: ColliderHandle,
        collider: &Collider,
    ) -> bool {
        self.scene.world.test_filter(filter, handle, collider)
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
        self.scene.world.cast_ray(ray, max_toi, solid, filter)
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
        self.scene.world.cast_shape(
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
        self.scene
            .world
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
        self.scene
            .world
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
        self.scene
            .world
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
        self.scene
            .world
            .intersection_with_shape(shape_pos, shape, filter)
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn intersections_with_point(
        &self,
        point: &Point<f32>,
        filter: QueryFilter,
        callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.scene
            .world
            .intersections_with_point(point, filter, callback)
    }

    #[inline]
    pub const fn total_time_duration(&self) -> Duration {
        self.shura.frame_manager.total_time_duration()
    }

    #[inline]
    pub fn total_time(&self) -> f32 {
        self.shura.frame_manager.total_time()
    }

    #[inline]
    pub const fn frame_time_duration(&self) -> Duration {
        self.shura.frame_manager.frame_time_duration()
    }

    #[inline]
    pub fn frame_time(&self) -> f32 {
        self.shura.frame_manager.frame_time()
    }

    #[inline]
    pub const fn fps(&self) -> u32 {
        self.shura.frame_manager.fps()
    }

    #[inline]
    pub fn max_fps(&self) -> Option<u32> {
        self.scene.render_config.max_fps()
    }

    #[inline]
    pub fn max_frame_time(&self) -> Option<Duration> {
        self.scene.render_config.max_frame_time()
    }

    #[inline]
    pub fn window(&self) -> &winit::window::Window {
        &self.shura.window
    }

    #[inline]
    pub fn window_mut(&mut self) -> &mut winit::window::Window {
        &mut self.shura.window
    }

    #[inline]
    pub fn group_mut(&mut self, id: u32) -> Option<&mut ComponentGroup> {
        if let Some(group_index) = self.scene.component_manager.group_index(&id) {
            return self.scene.component_manager.group_mut(*group_index);
        }
        return None;
    }

    #[inline]
    pub fn group(&self, id: u32) -> Option<&ComponentGroup> {
        if let Some(group_index) = self.scene.component_manager.group_index(&id) {
            return self.scene.component_manager.group(*group_index);
        }
        return None;
    }

    #[inline]
    pub fn scene_id(&self) -> u32 {
        self.scene.id()
    }

    #[inline]
    pub fn active_scene(&self) -> u32 {
        self.shura.scene_manager.active_scene()
    }

    #[inline]
    pub fn scene_ids(&self) -> impl Iterator<Item = &u32> {
        self.shura.scene_manager.scene_ids()
    }

    #[inline]
    pub fn does_scene_exist(&self, name: u32) -> bool {
        self.shura.scene_manager.does_scene_exist(name)
    }

    #[inline]
    pub fn active_group_ids(&self) -> &[u32] {
        self.scene.component_manager.active_group_ids()
    }

    #[inline]
    pub fn group_ids(&self) -> Vec<u32> {
        self.scene.component_manager.group_ids()
    }

    #[inline]
    pub fn component_dynamic(&self, handle: &ComponentHandle) -> Option<&DynamicComponent> {
        self.scene.component_manager.component_dynamic(handle)
    }

    pub fn component_dynamic_mut(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut DynamicComponent> {
        self.scene.component_manager.component_dynamic_mut(handle)
    }

    #[inline]
    pub fn component<C: ComponentController>(&self, handle: &ComponentHandle) -> Option<&C> {
        self.scene.component_manager.component::<C>(handle)
    }

    #[inline]
    pub fn component_mut<C: ComponentController>(
        &mut self,
        handle: &ComponentHandle,
    ) -> Option<&mut C> {
        self.scene.component_manager.component_mut::<C>(handle)
    }

    #[inline]
    /// Force the position of the all component from the given generic to be updated inside the
    /// (InstanceBuffer)[crate::InstanceBuffer]. This is used when the [crate::ComponentConfig::does_move]
    /// flag is set, but the position needs to be updated.
    pub fn force_buffer<C: ComponentController + ComponentIdentifier>(&mut self) {
        self.scene.component_manager.force_buffer::<C>()
    }

    #[inline]
    /// Force the position of the components from the given groups from the given generic to be updated inside the
    /// (InstanceBuffer)[crate::InstanceBuffer]. This is used when the [crate::ComponentConfig::does_move]
    /// flag is set, but the position needs to be updated.
    pub fn force_buffer_groups<C: ComponentController + ComponentIdentifier>(
        &mut self,
        group_ids: &[u32],
    ) {
        self.scene
            .component_manager
            .force_buffer_groups::<C>(group_ids)
    }

    #[inline]
    /// Force the position of the active components from the given generic to be updated inside the
    /// (InstanceBuffer)[crate::InstanceBuffer]. This is used when the [crate::ComponentConfig::does_move]
    /// flag is set, but the position needs to be updated.
    pub fn force_buffer_active<C: ComponentController + ComponentIdentifier>(&mut self) {
        self.scene.component_manager.force_buffer_active::<C>()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn gravity(&self) -> &Vector<f32> {
        self.scene.world.gravity()
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn physics_priority(&self) -> i16 {
        self.scene.world.physics_priority()
    }

    #[inline]
    pub fn components_mut<C: ComponentController + ComponentIdentifier>(
        &'a mut self,
        filter: GroupFilter,
    ) -> ComponentSetMut<'a, C> {
        self.scene.component_manager.components_mut::<C>(filter)
    }

    #[inline]
    pub fn components<C: ComponentController + ComponentIdentifier>(
        &'a self,
        filter: GroupFilter,
    ) -> ComponentSet<'a, C> {
        self.scene.component_manager.components::<C>(filter)
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn gamepads(&self) -> Option<ConnectedGamepadsIterator> {
        self.shura.input.gamepads()
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
        self.shura.input.gamepad(gamepad_id)
    }

    //////////////////////////////////////////////////////////////////////////////////////////////
    // Setter
    //////////////////////////////////////////////////////////////////////////////////////////////

    #[inline]
    pub fn set_render_scale(&mut self, scale: f32) {
        self.scene.render_config.set_render_scale(self.shura, scale);
    }

    #[inline]
    pub fn set_active_scene(&mut self, active_scene: u32) {
        self.shura.scene_manager.set_active_scene(active_scene)
    }

    #[inline]
    pub fn set_update_components(&mut self, update_components: bool) {
        self.scene
            .component_manager
            .set_update_components(update_components)
    }

    #[inline]
    pub fn set_render_components(&mut self, render_components: bool) {
        self.scene
            .component_manager
            .set_render_components(render_components)
    }

    #[inline]
    pub fn set_camera_position(&mut self, pos: Isometry<f32>) {
        self.scene.camera.set_position(pos);
    }

    #[inline]
    pub fn set_camera_translation(&mut self, translation: Vector<f32>) {
        self.scene.camera.set_translation(translation);
    }

    #[inline]
    pub fn set_camera_rotation(&mut self, rotation: Rotation<f32>) {
        self.scene.camera.set_rotation(rotation);
    }

    pub fn set_camera_target(&mut self, target: Option<ComponentHandle>) {
        self.scene.camera.set_target(target);
    }

    #[inline]
    /// Tries to enable or disable vSync. The default is always vSync to be on.
    /// So every device supports vSync but not every device supports no vSync.
    pub fn set_vsync(&mut self, vsync: bool) {
        self.shura.gpu.set_vsync(vsync);
    }

    #[inline]
    pub fn set_cursor_hidden(&mut self, hidden: bool) {
        self.shura.window.set_cursor_visible(!hidden);
    }

    #[inline]
    /// Set the distance between the center of the camera to the top in world coordinates.
    pub fn set_vertical_fov(&mut self, fov: f32) {
        let window_size = self.window_size();
        self.scene.camera.set_vertical_fov(
            &mut self.scene.cursor,
            &self.shura.input,
            window_size,
            fov,
        );
    }

    #[inline]
    /// Set the distance between the center of the camera to the right in world coordinates.
    pub fn set_horizontal_fov(&mut self, fov: f32) {
        let window_size = self.window_size();
        self.scene.camera.set_horizontal_fov(
            &mut self.scene.cursor,
            &self.shura.input,
            window_size,
            fov,
        );
    }

    #[inline]
    pub fn set_window_size(&mut self, size: Dimension<u32>) {
        let size: winit::dpi::PhysicalSize<u32> = size.into();
        self.shura.window.set_inner_size(size);
    }

    #[inline]
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        if fullscreen {
            let f = winit::window::Fullscreen::Borderless(None);
            self.shura.window.set_fullscreen(Some(f));
        } else {
            self.shura.window.set_fullscreen(None);
        }
    }

    #[inline]
    pub fn set_clear_color(&mut self, color: Option<Color>) {
        self.scene.render_config.set_clear_color(color);
    }

    #[inline]
    pub fn set_window_resizable(&mut self, resizable: bool) {
        self.shura.window.set_resizable(resizable);
    }

    #[inline]
    pub fn set_window_title(&mut self, title: &str) {
        self.shura.window.set_title(title);
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.scene.world.set_gravity(gravity);
    }

    #[inline]
    #[cfg(feature = "physics")]
    pub fn set_physics_priority(&mut self, step: i16) {
        self.scene.world.set_physics_priority(step);
    }

    // #[inline]
    // #[cfg(feature = "gamepad")]
    // pub fn set_gamepad_mapping(
    //     &mut self,
    //     gamepad_id: GamepadId,
    //     mapping: &Mapping,
    //     name: Option<&str>,
    // ) -> Result<String, MappingError> {
    //     self.shura
    //         .input
    //         .set_gamepad_mapping(gamepad_id, mapping, name)
    // }

    // #[inline]
    // #[cfg(feature = "gamepad")]
    // pub fn set_gamepad_mapping_strict(
    //     &mut self,
    //     gamepad_id: GamepadId,
    //     mapping: &Mapping,
    //     name: Option<&str>,
    // ) -> Result<String, MappingError> {
    //     self.shura
    //         .input
    //         .set_gamepad_mapping_strict(gamepad_id, mapping, name)
    // }

    #[inline]
    pub fn set_max_fps(&mut self, max_fps: Option<u32>) {
        self.scene.render_config.set_max_fps(max_fps);
    }
}
