use crate::{
    CameraBuffer,  GpuDefaults, InstanceBuffer, InstanceIndices, Model,
    RenderEncoder, Shader, Sprite, Uniform, RenderTarget, 
};

/// Render grpahics to the screen or a sprite. The renderer can be extended with custom graphcis throught
/// the [RenderPass](wgpu::RenderPass) or the provided methods for shura's shader system.
pub struct Renderer<'a> {
    pub render_pass: wgpu::RenderPass<'a>,
    pub indices: u32,
    pub msaa: bool,
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
}
