#[cfg(feature = "text")]
use crate::text::Text;

use crate::{
    Camera2D, Color, DefaultResources, Gpu, Instance, InstanceBuffer, InstanceBuffer2D,
    InstanceIndices, Model, Model2D, RenderTarget, Shader, Sprite, SpriteSheet, Uniform, Vertex,
};
use std::{ops::Range, ptr::null};

struct RenderCache {
    pub bound_shader: *const Shader,
    pub bound_buffers: [*const wgpu::Buffer; 3],
    pub bound_uniforms: [*const wgpu::BindGroup; 16],
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            bound_shader: null(),
            bound_buffers: [null(); 3],

            bound_uniforms: [null(); 16],
        }
    }
}

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub(crate) target: &'a dyn RenderTarget,
    pub gpu: &'a Gpu,
    pub defaults: &'a DefaultResources,
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
        defaults: &'a DefaultResources,
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
            indices: 0,
            render_pass,
            defaults,
            target,
            gpu,
            cache: RenderCache::default(),
        };
    }

    pub fn target(&self) -> &dyn RenderTarget {
        self.target
    }

    pub fn pass(&'a mut self) -> &mut wgpu::RenderPass {
        self.cache = Default::default();
        return &mut self.render_pass;
    }

    /// Sets the instance buffer at the position 1
    pub fn use_instances<I: Instance>(&mut self, buffer: &'a InstanceBuffer<I>) {
        let ptr = buffer.buffer() as *const _;
        if self.cache.bound_buffers[Self::INSTANCE_SLOT as usize] != ptr {
            self.cache.bound_buffers[Self::INSTANCE_SLOT as usize] = ptr;
            self.render_pass
                .set_vertex_buffer(Self::INSTANCE_SLOT, buffer.slice());
        }
    }

    pub fn use_camera(&mut self, camera: &'a Camera2D) {
        self.use_bind_group(camera.bindgroup(), Self::CAMERA_SLOT)
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        let ptr = shader as *const _;
        if ptr != self.cache.bound_shader {
            self.cache.bound_shader = ptr;
            self.render_pass.set_pipeline(shader.pipeline());
        }
    }

    pub fn use_shader_with_buffers<I: Instance, T: Vertex>(
        &mut self,
        shader: &'a Shader,
        instances: &'a InstanceBuffer<I>,
        model: &'a Model<T>,
    ) {
        debug_assert_eq!(shader.instance_size(), instances.instance_size());
        debug_assert_eq!(shader.vertex_size(), model.vertex_size());
        self.use_shader(shader);
        self.use_model(model);
        self.use_instances(instances);
    }

    pub fn use_model<T: Vertex>(&mut self, model: &'a Model<T>) {
        let ptr = model.buffer() as *const _;
        if self.cache.bound_buffers[Self::MODEL_SLOT as usize] != ptr {
            self.cache.bound_buffers[Self::MODEL_SLOT as usize] = ptr;
            self.indices = model.index_amount();
            self.render_pass
                .set_index_buffer(model.index_buffer(), wgpu::IndexFormat::Uint32);
            self.render_pass
                .set_vertex_buffer(Self::MODEL_SLOT, model.vertex_buffer());
        }
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
            .draw_indexed(indices, 0, instances.into().range());
    }

    pub fn render_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        model: &'a Model2D,
        sprite: &'a Sprite,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.sprite, buffer, model);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.draw(instances);
        }
    }

    pub fn render_sprite_sheet(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        model: &'a Model2D,
        sprite: &'a SpriteSheet,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.sprite_sheet, buffer, model);
            self.use_camera(camera);
            self.use_sprite_sheet(sprite, 1);
            self.draw(instances);
        }
    }

    pub fn render_color(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        model: &'a Model2D,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.color, buffer, model);
            self.use_camera(camera);
            self.draw(instances);
        }
    }

    #[cfg(feature = "text")]
    pub fn render_text(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        text: &'a Text,
    ) {
        if buffer.buffer_size() != 0 && text.model().vertex_buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.text, buffer, text.model());
            self.use_camera(camera);
            self.use_model(text.model());
            self.use_sprite_sheet(text.font(), 1);
            self.draw(instances);
        }
    }

    pub fn render_grey(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        model: &'a Model2D,
        sprite: &'a Sprite,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.grey, buffer, model);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.draw(instances);
        }
    }

    pub fn render_blurred(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        model: &'a Model2D,
        sprite: &'a Sprite,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.blurr, buffer, model);
            self.use_camera(camera);
            self.use_sprite(sprite, 1);
            self.draw(instances);
        }
    }

    pub fn render_rainbow(
        &mut self,
        instances: impl Into<InstanceIndices>,
        buffer: &'a InstanceBuffer2D,
        camera: &'a Camera2D,
        model: &'a Model2D,
    ) {
        if buffer.buffer_size() != 0 {
            self.use_shader_with_buffers(&self.defaults.rainbow, buffer, model);
            self.use_camera(camera);
            self.use_uniform(&self.defaults.times, 1);
            self.draw(instances);
        }
    }
}
