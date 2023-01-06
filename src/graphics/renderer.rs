use crate::{Camera, Color, Gpu, InstanceBuffer, Matrix, Model, Shader, Sprite, Uniform};

/// Single index of an instance inside a [InstanceBuffer](crate::InstanceBuffer).
pub type Instance = u32;
/// Range of [instances](crate::Instance).
pub type Instances = std::ops::Range<Instance>;

trait CopyInstance {
    fn copy(&self) -> Self;
}

impl CopyInstance for Instances {
    fn copy(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
        }
    }
}

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught 
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub gpu: &'a Gpu,
    indices: u32,
    pub save_sprite: Option<String>,
}

impl<'a> Renderer<'a> {
    pub(crate) fn new(
        encoder: &'a mut wgpu::CommandEncoder,
        target: &'a wgpu::TextureView,
        msaa: &'a wgpu::TextureView,
        gpu: &'a Gpu,
    ) -> Renderer<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa,
                resolve_target: Some(target),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });

        Self {
            render_pass,
            gpu,
            indices: 0,
            save_sprite: None,
        }
    }

    pub(crate) fn clear(
        encoder: &'a mut wgpu::CommandEncoder,
        target: &'a wgpu::TextureView,
        msaa: &'a wgpu::TextureView,
        color: Color,
    ) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("compute_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa,
                resolve_target: Some(&target),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color.into()),
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });
    }

    pub(crate) fn new_compute(
        encoder: &'a mut wgpu::CommandEncoder,
        gpu: &'a Gpu,
        target: &'a wgpu::TextureView,
        msaa: &'a wgpu::TextureView,
        instances: &'a InstanceBuffer,
        camera: &'a Uniform<Matrix>,
    ) -> Renderer<'a> {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("compute_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa,
                resolve_target: Some(target),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });

        let mut ctx = Self {
            render_pass,
            gpu,
            indices: 0,
            save_sprite: None,
        };

        ctx.use_uniform(camera, 0);
        ctx.set_instance_buffer(instances);
        return ctx;
    }

    /// Sets the instance buffer at the position 1
    pub fn set_instance_buffer(&mut self, buffer: &'a InstanceBuffer) {
        self.render_pass.set_vertex_buffer(1, buffer.slice());
    }

    pub(crate) fn enable_camera(&mut self, camera: &'a Camera) {
        self.render_pass
            .set_bind_group(0, camera.uniform().bind_group(), &[]);
    }

    /// Save the current render after finishing the current function onto a new [Sprite]. This does only work
    /// after rendering all components of a type or after postprocessing. The saved sprites
    pub fn save_sprite(&mut self, target_sprite: String) {
        self.save_sprite = Some(target_sprite);
    }

    pub fn use_shader(&mut self, shader: &'a Shader) {
        self.render_pass.set_pipeline(shader.pipeline());
    }

    pub fn use_model(&mut self, model: &'a Model) {
        self.render_pass
            .set_index_buffer(model.index_buffer().slice(..), wgpu::IndexFormat::Uint16);
        self.render_pass
            .set_vertex_buffer(0, model.vertex_buffer().slice(..));
        self.indices = model.amount_of_indices();
    }

    pub fn use_sprite(&mut self, sprite: &'a Sprite, slot: u32) {
        self.render_pass
            .set_bind_group(slot, sprite.bind_group(), &[]);
    }

    pub fn use_color(&mut self, color: &'a Uniform<Color>, slot: u32) {
        self.render_pass
            .set_bind_group(slot, color.bind_group(), &[]);
    }

    pub fn use_uniform<T: bytemuck::Pod>(&mut self, uniform: &'a Uniform<T>, slot: u32) {
        self.render_pass
            .set_bind_group(slot, uniform.bind_group(), &[]);
    }

    /// This uniform stores both the total time and the frame time.
    /// ```
    /// struct Times {
    ///     total_time: f32,
    ///     delta_time: f32
    /// }
    ///
    /// @group(1) @binding(0)
    /// var<uniform> total_time: Times;
    /// ```
    pub fn use_time_uniform(&mut self, slot: u32) {
        self.render_pass
            .set_bind_group(slot, self.gpu.defaults.times.bind_group(), &[]);
    }

    pub fn render_sprite(&mut self, model: &'a Model, sprite: &'a Sprite) {
        self.use_shader(&self.gpu.defaults.sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
    }

    pub fn render_grey(&mut self, model: &'a Model, sprite: &'a Sprite) {
        self.use_shader(&self.gpu.defaults.grey);
        self.use_model(model);
        self.use_sprite(sprite, 1);
    }

    pub fn render_blurred(&mut self, model: &'a Model, sprite: &'a Sprite) {
        self.use_shader(&self.gpu.defaults.blurr);
        self.use_model(model);
        self.use_sprite(sprite, 1);
    }

    pub fn render_colored_sprite(
        &mut self,
        model: &'a Model,
        sprite: &'a Sprite,
        color: &'a Uniform<Color>,
    ) {
        self.use_shader(&self.gpu.defaults.colored_sprite);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_color(color, 2);
    }

    pub fn render_transparent_sprite(
        &mut self,
        model: &'a Model,
        sprite: &'a Sprite,
        transparency: &'a Uniform<f32>,
    ) {
        self.use_shader(&self.gpu.defaults.transparent);
        self.use_model(model);
        self.use_sprite(sprite, 1);
        self.use_uniform(transparency, 2)
    }

    pub fn render_color(&mut self, model: &'a Model, color: &'a Uniform<Color>) {
        self.use_shader(&self.gpu.defaults.color);
        self.use_model(model);
        self.use_color(color, 1);
    }

    pub fn render_rainbow(&mut self, model: &'a Model) {
        self.use_shader(&self.gpu.defaults.rainbow);
        self.use_model(model);
        self.use_uniform(&self.gpu.defaults.times, 1);
    }

    // pub fn render_cropped(&mut self, model: &'a Model) {

    // }

    #[inline]
    pub fn commit(&mut self, instances: &Instances) {
        self.render_pass
            .draw_indexed(0..self.indices, 0, instances.copy());
    }

    #[inline]
    pub fn commit_one(&mut self, index: Instance) {
        self.render_pass
            .draw_indexed(0..self.indices, 0, index..index + 1);
    }

    // Getter
    #[inline]
    pub const fn gpu(&self) -> &Gpu {
        &self.gpu
    }

    #[inline]
    pub fn render_pass(&mut self) -> &mut wgpu::RenderPass<'a> {
        &mut self.render_pass
    }
}
