use crate::{
    CameraBuffer, Color, GpuDefaults, InstanceBuffer, InstanceIndices, Model, RenderEncoder,
    RenderTarget, Shader, Sprite, Uniform,
};
use std::ptr::null;

struct RenderCache {
    pub bound_shader: *const Shader,
    pub bound_camera: *const CameraBuffer,
    pub bound_model: *const Model,
    pub bound_instances: *const InstanceBuffer,
    pub bound_uniforms: [*const wgpu::BindGroup; 5],
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            bound_shader: null(),
            bound_camera: null(),
            bound_model: null(),
            bound_instances: null(),
            bound_uniforms: [null(), null(), null(), null(), null()],
        }
    }
}

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub indices: u32,
    pub msaa: bool,
    cache: RenderCache,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(
        render_target: &'a RenderTarget,
        render_encoder: &'a mut RenderEncoder,
    ) -> Renderer<'a> {
        let render_pass = render_encoder
            .inner
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: if render_encoder.msaa {
                        render_target.msaa()
                    } else {
                        render_target.view()
                    },
                    resolve_target: if render_encoder.msaa {
                        Some(render_target.view())
                    } else {
                        None
                    },
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],

                depth_stencil_attachment: None,
            });

        Self {
            render_pass,
            indices: 0,
            msaa: render_encoder.msaa,
            cache: Default::default(),
        }
    }

    pub(crate) fn output_renderer(
        encoder: &'a mut wgpu::CommandEncoder,
        defaults: &'a GpuDefaults,
        output: &'a wgpu::TextureView,
    ) -> Renderer<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });

        let mut renderer = Self {
            render_pass,
            indices: 0,
            msaa: false,
            cache: Default::default(),
        };
        renderer.use_uniform(defaults.relative_camera.buffer().uniform(), 0);
        renderer.use_instances(&defaults.single_centered_instance);
        return renderer;
    }

    /// Sets the instance buffer at the position 1
    pub fn use_instances(&mut self, buffer: &'a InstanceBuffer) {
        let ptr = buffer as *const _;
        if ptr != self.cache.bound_instances {
            self.cache.bound_instances = ptr;
            self.render_pass.set_vertex_buffer(1, buffer.slice());
        }
    }

    pub fn use_camera(&mut self, camera: &'a CameraBuffer) {
        let ptr = camera as *const _;
        if ptr != self.cache.bound_camera {
            self.cache.bound_camera = ptr;
            self.render_pass
                .set_bind_group(0, camera.uniform().bind_group(), &[]);
        }
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        assert_eq!(
            shader.msaa(),
            self.msaa,
            "The Renderer and the Shader both need to have msaa enabled / disabled!"
        );
        let ptr = shader as *const _;
        if ptr != self.cache.bound_shader {
            self.cache.bound_shader = ptr;
            self.render_pass.set_pipeline(shader.pipeline());
        }
    }

    pub fn use_model(&mut self, model: &'a Model) {
        let ptr = model as *const _;
        if ptr != self.cache.bound_model {
            self.cache.bound_model = ptr;

            self.render_pass
                .set_index_buffer(model.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
            self.render_pass
                .set_vertex_buffer(0, model.vertex_buffer().slice(..));
            self.indices = model.amount_of_indices();
        }
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.use_bind_group(sprite.bind_group(), slot);
    }

    pub fn use_uniform<T: bytemuck::Pod>(&mut self, uniform: &'a Uniform<T>, slot: u32) {
        self.use_bind_group(uniform.bind_group(), slot);
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

    pub fn draw(&mut self, instances: impl Into<InstanceIndices>) {
        self.render_pass
            .draw_indexed(0..self.indices, 0, instances.into().range);
    }

    pub const fn msaa(&self) -> bool {
        self.msaa
    }

    pub fn render_pass(&mut self) -> &mut wgpu::RenderPass<'a> {
        &mut self.render_pass
    }

    pub fn render_sprite(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_grey(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.grey);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_blurred(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.blurr);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_colored_sprite(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
        color: &'a Uniform<Color>,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.colored_sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(color, 2);
        self.draw(instances);
    }

    pub fn render_transparent_sprite(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
        transparency: &'a Uniform<f32>,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.transparent);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(transparency, 2);
        self.draw(instances);
    }

    pub fn render_color(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.color);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.draw(instances);
    }

    pub fn render_color_no_msaa(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.color_no_msaa);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.draw(instances);
    }

    pub fn render_sprite_no_msaa(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.sprite_no_msaa);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.draw(instances);
    }

    pub fn render_rainbow(
        &mut self,
        defaults: &'a GpuDefaults,
        instance_buffer: &'a InstanceBuffer,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
    ) {
        self.use_instances(instance_buffer);
        self.use_shader(&defaults.rainbow);
        self.use_model(model);
        self.use_uniform(&defaults.times, 1);
        self.draw(instances);
    }
}
