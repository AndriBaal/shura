use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, InstanceIndices, Model,
    RenderEncoder, Shader, Sprite, Uniform, 
};

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
    pub indices: u32,
    pub msaa: bool,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(
        render_encoder: &'a mut RenderEncoder,
    ) -> Renderer<'a> {
        let render_pass = render_encoder
            .inner
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: if render_encoder.msaa {
                        render_encoder.target.msaa()
                    } else {
                        render_encoder.target.view()
                    },
                    resolve_target: if render_encoder.msaa {
                        Some(render_encoder.target.view())
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

        let mut result = Self {
            render_pass,
            gpu: render_encoder.gpu,
            defaults: render_encoder.defaults,
            indices: 0,
            msaa: render_encoder.msaa,
        };
        return result;
    }

    pub(crate) fn output_renderer(
        encoder: &'a mut wgpu::CommandEncoder,
        gpu: &'a Gpu,
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
            gpu,
            defaults,
            indices: 0,
            msaa: false,
        };
        renderer.use_uniform(defaults.relative_camera.buffer().uniform(), 0);
        renderer.use_instances(&defaults.single_centered_instance);
        return renderer;
    }

    /// Sets the instance buffer at the position 1
    pub fn use_instances(&mut self, buffer: &'a InstanceBuffer) -> InstanceIndices {
        self.render_pass.set_vertex_buffer(1, buffer.slice());
        return buffer.instances();
    }

    pub fn use_camera(&mut self, camera: &'a CameraBuffer) {
        self.render_pass
            .set_bind_group(0, camera.uniform().bind_group(), &[]);
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        assert_eq!(
            shader.msaa(),
            self.msaa,
            "The Renderer and the Shader both need to have msaa enabled / disabled!"
        );
        self.render_pass.set_pipeline(shader.pipeline());
    }

    pub fn use_model(&mut self, model: &'a Model) {
        self.render_pass
            .set_index_buffer(model.index_buffer().slice(..), wgpu::IndexFormat::Uint32);
        self.render_pass
            .set_vertex_buffer(0, model.vertex_buffer().slice(..));
        self.indices = model.amount_of_indices();
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.render_pass
            .set_bind_group(slot, sprite.bind_group(), &[]);
    }

    pub fn use_uniform<T: bytemuck::Pod>(&mut self, uniform: &'a Uniform<T>, slot: u32) {
        self.render_pass
            .set_bind_group(slot, uniform.bind_group(), &[]);
    }

    /// This uniform stores both the total time and the frame time.
    /// ```
    /// struct Times {
    ///     total_time: f32,
    ///     frame_time: f32
    /// }
    ///
    /// @group(1) @binding(0)
    /// var<uniform> total_time: Times;
    /// ```
    pub fn use_time_uniform(&mut self, slot: u32) {
        self.render_pass
            .set_bind_group(slot, self.defaults.times.bind_group(), &[]);
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
        self.commit(instances);
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
        self.commit(instances);
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
        self.commit(instances);
    }

    pub fn render_colored_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.colored_sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(color, 2);
        self.commit(instances);
    }

    pub fn render_transparent_sprite(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
        transparency: &'a Uniform<f32>,
    ) {
        self.use_shader(&self.defaults.transparent);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(transparency, 2);
        self.commit(instances);
    }

    pub fn render_color(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.color);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.commit(instances);
    }

    pub fn render_color_no_msaa(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.defaults.color_no_msaa);
        self.use_model(model);
        self.use_uniform(color, 1);
        self.commit(instances);
    }

    pub fn render_sprite_no_msaa(
        &mut self,
        instances: impl Into<InstanceIndices>,
        model: &'a Model,
        sprite: &'a Sprite,
    ) {
        self.use_shader(&self.defaults.sprite_no_msaa);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.commit(instances);
    }

    pub fn render_rainbow(&mut self, instances: impl Into<InstanceIndices>, model: &'a Model) {
        self.use_shader(&self.defaults.rainbow);
        self.use_model(model);
        self.use_uniform(&self.defaults.times, 1);
        self.commit(instances);
    }

    pub fn commit(&mut self, instances: impl Into<InstanceIndices>) {
        self.render_pass
            .draw_indexed(0..self.indices, 0, instances.into().range);
    }

    pub const fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    pub const fn msaa(&self) -> bool {
        self.msaa
    }

    pub fn render_pass(&mut self) -> &mut wgpu::RenderPass<'a> {
        &mut self.render_pass
    }
}
