#[cfg(feature = "text")]
use crate::text::TextDescriptor;
use crate::{
    Color, Gpu, GpuDefaults, RenderTarget, Renderer,
};

pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub gpu: &'a Gpu,
    pub defaults: &'a GpuDefaults,
    pub target: &'a RenderTarget,
    pub msaa: bool,
}

impl <'a>RenderEncoder<'a> {
    pub(crate) fn new(gpu: &'a Gpu, defaults: &'a GpuDefaults, target: &'a RenderTarget) -> Self {
        let inner = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        Self { inner, defaults, gpu, target, msaa: true }
    }

    pub fn clear(&mut self, color: Color) {
        self.inner.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.target.msaa(),
                resolve_target: Some(self.target.view()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color.into()),
                    store: true,
                },
            })],

            depth_stencil_attachment: None,
        });
    }

    pub fn renderer(
        &'a mut self,
    ) -> Renderer<'a> {
        Renderer::new(self)
    }

    #[cfg(feature = "text")]
    pub fn render_text(&mut self,  descriptor: TextDescriptor) {
        let target_size = self.target.size();
        let mut staging_belt = wgpu::util::StagingBelt::new(1024);
        if let Some(color) = descriptor.clear_color {
            self.clear(color);
        }

        for section in descriptor.sections {
            descriptor.font.brush.queue(section.to_glyph_section());
        }

        descriptor
            .font
            .brush
            .draw_queued(
                &self.gpu.device,
                &mut staging_belt,
                &mut self.inner,
                self.target.view(),
                2 * target_size.x,
                2 * target_size.y,
            )
            .expect("Draw queued");

        staging_belt.finish();
    }

    pub fn copy_target(&mut self, into: &RenderTarget) {
        let prev_target = self.target;
        let defaults = self.defaults;
        let mut renderer = Renderer::new(self);
        renderer.use_camera(&defaults.relative_camera);
        renderer.use_instances(&defaults.single_centered_instance);
        renderer.render_sprite(
            0,
            defaults.relative_camera.model(),
            defaults.target.sprite(),
        );
        self.target = prev_target;
    }

    pub fn submit(self, gpu: &Gpu) {
        gpu.queue.submit(Some(self.inner.finish()));
    }
}
