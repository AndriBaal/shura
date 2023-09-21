#[cfg(feature = "text")]
use crate::text::Text;

use crate::{
    Camera, Color, Component, ComponentHandle, ComponentSetResource, Context, Gpu, GpuDefaults,
    GroupFilter, GroupHandle, InstanceBuffer, InstanceIndex, InstanceIndices, Model, RenderTarget,
    Shader, Sprite, SpriteRenderTarget, SpriteSheet, SpriteSheetIndex, Uniform,
};
use std::{ops::Range, ptr::null};

struct RenderCache {
    pub bound_shader: *const Shader,
    pub bound_model: *const Model,
    #[cfg(feature = "text")]
    pub bound_text: *const Text,
    pub bound_instances: *const InstanceBuffer,
    pub bound_uniforms: [*const wgpu::BindGroup; 16],
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            bound_shader: null(),
            bound_model: null(),
            bound_instances: null(),
            bound_uniforms: [null(); 16],
            #[cfg(feature = "text")]
            bound_text: null(),
        }
    }
}

#[non_exhaustive]
pub struct ComponentRenderer<'a> {
    pub inner: Renderer<'a>,
    pub screenshot: Option<&'a SpriteRenderTarget>,
    pub ctx: &'a Context<'a>,
    pub world_camera: &'a Camera,
    pub relative_camera: &'a Camera,
    pub relative_bottom_left_camera: &'a Camera,
    pub relative_bottom_right_camera: &'a Camera,
    pub relative_top_left_camera: &'a Camera,
    pub relative_top_right_camera: &'a Camera,
    pub single_centered_instance: &'a InstanceBuffer,
    pub unit_camera: &'a Camera,
}

impl<'a> ComponentRenderer<'a> {
    pub(crate) fn new(ctx: &'a Context<'a>, inner: Renderer<'a>) -> Self {
        ComponentRenderer {
            ctx: &ctx,
            screenshot: None,
            inner,
            world_camera: ctx.world_camera,
            relative_camera: &ctx.defaults.relative_camera,
            relative_bottom_left_camera: &ctx.defaults.relative_bottom_left_camera,
            relative_bottom_right_camera: &ctx.defaults.relative_bottom_right_camera,
            relative_top_left_camera: &ctx.defaults.relative_top_left_camera,
            relative_top_right_camera: &ctx.defaults.relative_top_right_camera,
            unit_camera: &ctx.defaults.unit_camera,
            single_centered_instance: &ctx.defaults.single_centered_instance,
        }
    }

    pub fn for_each<C: Component>(&self, each: impl FnMut(&C) + 'a) {
        let ty = self.ctx.components.resource();
        ty.for_each(self.ctx.components.active_groups(), each);
    }

    pub fn unit_model(&self) -> &'a Model {
        self.inner.defaults.unit_model()
    }

    pub fn default_target(&self) -> &'a dyn RenderTarget {
        self.inner.defaults.default_target()
    }

    pub fn index<C: Component>(&self, group: GroupHandle, index: usize) -> Option<&'a C> {
        self.index_of(group, index)
    }

    pub fn index_of<C: Component>(&self, group: GroupHandle, index: usize) -> Option<&'a C> {
        let ty = self.ctx.components.resource();
        ty.index(group, index)
    }

    pub fn get<C: Component>(&self, handle: ComponentHandle) -> Option<&'a C> {
        let ty = self.ctx.components.resource();
        ty.get(handle)
    }

    pub fn len<C: Component>(&self) -> usize {
        let ty = self.ctx.components.resource::<C>();
        ty.len(self.ctx.components.active_groups())
    }

    pub fn iter<C: Component>(&self) -> impl DoubleEndedIterator<Item = &'a C> {
        let ty = self.ctx.components.resource();
        ty.iter(self.ctx.components.active_groups())
    }

    pub fn iter_with_handles<C: Component>(
        &self,
    ) -> impl DoubleEndedIterator<Item = (ComponentHandle, &'a C)> {
        let ty = self.ctx.components.resource();
        ty.iter_with_handles(self.ctx.components.active_groups())
    }

    pub fn try_single<C: Component>(&self) -> Option<&'a C> {
        let ty = self.ctx.components.resource();
        ty.try_single()
    }

    pub fn single<C: Component>(&self) -> &'a C {
        let ty = self.ctx.components.resource();
        ty.single()
    }

    pub fn resource<C: Component>(&self) -> ComponentSetResource<'a, C> {
        let ty = self.ctx.components.resource();
        return ComponentSetResource::new(ty, self.ctx.components.active_groups());
    }

    pub fn resource_of<C: Component>(
        &self,
        filter: GroupFilter<'a>,
    ) -> ComponentSetResource<'a, C> {
        let ty = self.ctx.components.resource();
        let groups = self.ctx.components.group_filter(filter);
        return ComponentSetResource::new(ty, groups);
    }

    pub fn render_each<C: Component>(
        &mut self,
        camera: &'a Camera,
        each: impl FnMut(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        let ty = self.ctx.components.resource::<C>();
        ty.render_each(&mut self.inner, camera, each)
    }

    #[cfg(feature = "rayon")]
    pub fn par_for_each<C: Component + Send + Sync>(&self, each: impl Fn(&C) + Send + Sync) {
        let ty = self.ctx.components.resource::<C>();
        ty.par_for_each(self.ctx.components.active_groups(), each);
    }

    pub fn render_single<C: Component>(
        &mut self,
        camera: &'a Camera,
        each: impl FnOnce(&mut Renderer<'a>, &'a C, InstanceIndex),
    ) {
        let ty = self.ctx.components.resource::<C>();
        ty.render_single(&mut self.inner, camera, each)
    }

    pub fn render_all<C: Component>(
        &mut self,
        camera: &'a Camera,
        all: impl FnMut(&mut Renderer<'a>, InstanceIndices),
    ) {
        let ty = self.ctx.components.resource::<C>();
        ty.render_all(&mut self.inner, camera, all)
    }
}

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
    pub(crate) target: &'a dyn RenderTarget,
    indices: u32,
    render_pass: wgpu::RenderPass<'a>,
    cache: RenderCache,
}

impl<'a> Renderer<'a> {
    pub const MODEL_SLOT: u32 = 0;
    pub const INSTANCE_SLOT: u32 = 1;
    pub const CAMERA_SLOT: u32 = 0;
    pub fn new(
        render_encoder: &'a mut wgpu::CommandEncoder,
        defaults: &'a GpuDefaults,
        gpu: &'a Gpu,
        target: &'a dyn RenderTarget,
        clear: Option<Color>,
    ) -> Renderer<'a> {
        let render_pass = render_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(target.attachment(clear))],

            depth_stencil_attachment: None,
        });

        return Self {
            render_pass,
            indices: 0,
            defaults: defaults,
            target,
            gpu,
            cache: RenderCache::default(),
        };
    }

    pub fn target(&self) -> &dyn RenderTarget {
        self.target
    }

    pub fn unit_model(&self) -> &'a Model {
        self.defaults.unit_model()
    }

    pub fn default_target(&self) -> &'a dyn RenderTarget {
        self.defaults.default_target()
    }

    pub fn pass(&'a mut self) -> &mut wgpu::RenderPass {
        self.cache = Default::default();
        return &mut self.render_pass;
    }

    /// Sets the instance buffer at the position 1
    pub fn use_instances(&mut self, buffer: &'a InstanceBuffer) {
        let ptr = buffer as *const _;
        if ptr != self.cache.bound_instances {
            self.cache.bound_instances = ptr;
            self.render_pass
                .set_vertex_buffer(Self::INSTANCE_SLOT, buffer.slice());
        }
    }

    pub fn use_camera(&mut self, camera: &'a Camera) {
        self.use_bind_group(camera.bindgroup(), Self::CAMERA_SLOT)
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        let ptr = shader as *const _;
        if ptr != self.cache.bound_shader {
            self.cache.bound_shader = ptr;
            self.render_pass.set_pipeline(shader.pipeline());
        }
    }

    pub fn use_model(&mut self, model: &'a Model) {
        let ptr = model as *const _;
        #[cfg(feature = "text")]
        {
            self.cache.bound_text = null();
        }
        if ptr != self.cache.bound_model {
            self.cache.bound_model = ptr;
            self.indices = model.amount_of_indices();
            self.render_pass
                .set_index_buffer(model.index_buffer(), wgpu::IndexFormat::Uint32);
            self.render_pass
                .set_vertex_buffer(Self::MODEL_SLOT, model.vertex_buffer());
        }
    }

    #[cfg(feature = "text")]
    pub fn use_text(&mut self, text: &'a Text) {
        let ptr = text as *const _;
        self.cache.bound_model = null();
        if ptr != self.cache.bound_text {
            self.cache.bound_text = ptr;
            self.indices = text.amount_of_indices();
            self.render_pass
                .set_index_buffer(text.index_buffer(), wgpu::IndexFormat::Uint32);
            self.render_pass
                .set_vertex_buffer(Self::MODEL_SLOT, text.vertex_buffer());
        }
        self.use_sprite_sheet(text.font(), 1);
    }

    pub fn use_bind_group(&mut self, bind_group: &'a wgpu::BindGroup, slot: u32) {
        let ptr = bind_group as *const _;
        if let Some(cache_slot) = self.cache.bound_uniforms.get_mut(slot as usize) {
            if *cache_slot != ptr {
                *cache_slot = ptr;
                self.render_pass.set_bind_group(slot, bind_group, &[]);
            }
        } else {
            self.render_pass.set_bind_group(slot, bind_group, &[]);
        }
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.use_bind_group(sprite.bind_group(), slot);
    }

    pub fn use_sprite_sheet(&mut self, sprite_sheet: &'a SpriteSheet, slot: u32) {
        self.use_bind_group(sprite_sheet.bind_group(), slot);
    }

    pub fn use_uniform<T: bytemuck::Pod>(&mut self, uniform: &'a Uniform<T>, slot: u32) {
        self.use_bind_group(uniform.bind_group(), slot);
    }

    pub fn draw(&mut self, instances: impl Into<InstanceIndices>) {
        self.draw_indexed(0..self.indices, instances);
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, instances: impl Into<InstanceIndices>) {
        self.render_pass
            .draw_indexed(indices, 0, instances.into().range);
    }

    pub fn render_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_crop(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.sprite_crop);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_sheet_crop(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.sprite_sheet_crop);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_sheet(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a SpriteSheet,
    ) {
        self.use_shader(&self.defaults.sprite_sheet);
        self.use_model(model);
        self.use_sprite_sheet(sprite, 1);
        self.draw(instances);
    }

    pub fn render_sprite_sheet_uniform(
        &mut self,
        instances: impl Into<InstanceIndices> + Clone,
        model: &'a Model,
        sprite: &'a SpriteSheet,
        sprite_index: &'a Uniform<SpriteSheetIndex>,
    ) {
        self.use_shader(&self.defaults.sprite_sheet_uniform);
        self.use_model(model);
        self.use_sprite_sheet(sprite, 1);
        self.use_uniform(sprite_index, 2);
        self.draw(instances);
    }

    pub fn render_color(&mut self, instances: impl Into<InstanceIndices>, model: &'a Model) {
        self.use_shader(&self.defaults.color);
        self.use_model(model);
        self.draw(instances);
    }

    #[cfg(feature = "text")]
    pub fn render_text(&mut self, instances: impl Into<InstanceIndices>, text: &'a Text) {
        self.use_shader(&self.defaults.text);
        self.use_text(text);
        self.draw(instances);
    }

    pub fn render_color_uniform(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.color_uniform);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.draw(instances);
    }

    pub fn render_grey(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.grey);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_blurred(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.blurr);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_rainbow(&mut self, instances: impl Into<InstanceIndices>, model: &'a Model) {
        self.use_shader(&self.defaults.rainbow);
        self.use_model(model);
        self.use_uniform(&self.defaults.times, 1);
        self.draw(instances);
    }
}
