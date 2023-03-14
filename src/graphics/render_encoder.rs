use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, InstanceIndices, RenderTarget, Renderer,
};

#[derive(Copy, Clone)]
pub struct RenderConfig<'a> {
    pub camera: &'a CameraBuffer,
    pub instances: &'a InstanceBuffer,
    pub target: &'a RenderTarget,
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
    pub smaa: bool,
}

pub struct RenderEncoder {
    pub encoder: wgpu::CommandEncoder,
}

impl RenderEncoder {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        Self { encoder }
    }

    pub fn clear(&mut self, target: &RenderTarget, color: Color) {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.msaa(),
                resolve_target: Some(target.view()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color.into()),
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });
    }

    pub fn renderer<'a>(&'a mut self, config: RenderConfig<'a>) -> (InstanceIndices, Renderer<'a>) {
        Renderer::new(self, config)
    }

    pub fn copy_target(&mut self, config: RenderConfig, into: &RenderTarget) {
        let target_conf = RenderConfig {
            camera: &config.defaults.relative_camera,
            instances: &config.defaults.single_centered_instance,
            target: into,
            gpu: config.gpu,
            defaults: config.defaults,
            smaa: true,
        };

        let (instances, mut renderer) = Renderer::new(self, target_conf);
        renderer.render_sprite(
            config.defaults.relative_camera.model(),
            config.defaults.target.sprite(),
        );
        renderer.commit(instances);
    }
}
