#[cfg(feature = "text")]
use crate::text::TextDescriptor;
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
    pub msaa: bool,
}

pub struct RenderEncoder {
    pub inner: wgpu::CommandEncoder,
}

impl RenderEncoder {
    pub(crate) fn new(gpu: &Gpu) -> Self {
        let inner = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        Self { inner }
    }

    pub fn clear(&mut self, config: &RenderConfig, color: Color) {
        self.clear_target(config.target, color)
    }

    pub fn clear_target(&mut self, target: &RenderTarget, color: Color) {
        self.inner.begin_render_pass(&wgpu::RenderPassDescriptor {
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

    pub fn renderer<'a>(
        &'a mut self,
        config: &RenderConfig<'a>,
    ) -> (InstanceIndices, Renderer<'a>) {
        Renderer::new(self, config)
    }

    #[cfg(feature = "text")]
    pub fn render_text(&mut self, config: &RenderConfig, descriptor: TextDescriptor) {
        let gpu = config.gpu;
        let target = config.target;
        let target_size = target.size();
        let mut staging_belt = wgpu::util::StagingBelt::new(1024);
        if let Some(color) = descriptor.clear_color {
            self.clear_target(target, color);
        }

        for section in descriptor.sections {
            descriptor.font.brush.queue(section.to_glyph_section());
        }

        descriptor
            .font
            .brush
            .draw_queued(
                &gpu.device,
                &mut staging_belt,
                &mut self.inner,
                target.view(),
                2 * target_size.x,
                2 * target_size.y,
            )
            .expect("Draw queued");

        staging_belt.finish();
    }

    pub fn copy_target(&mut self, config: &RenderConfig, into: &RenderTarget) {
        let target_conf = RenderConfig {
            camera: &config.defaults.relative_camera,
            instances: &config.defaults.single_centered_instance,
            target: into,
            gpu: config.gpu,
            defaults: config.defaults,
            msaa: true,
        };

        let (instances, mut renderer) = Renderer::new(self, &target_conf);
        renderer.render_sprite(
            instances,
            config.defaults.relative_camera.model(),
            config.defaults.target.sprite(),
        );
    }

    pub fn submit(self, gpu: &Gpu) {
        gpu.queue.submit(Some(self.inner.finish()));
    }
}
