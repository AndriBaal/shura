#[cfg(feature = "text")]
use crate::text::TextDescriptor;
use crate::{CameraBuffer, Color, Gpu, GpuDefaults, RenderTarget, Renderer, Sprite};

pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub msaa: bool,
    pub defaults: &'a GpuDefaults
}

impl <'a>RenderEncoder<'a> {
    pub(crate) fn new(gpu: &Gpu,defaults: &'a GpuDefaults) -> Self {
        let inner = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });
        Self { inner, msaa: true, defaults }
    }

    pub fn clear(&mut self, target: &RenderTarget, color: Color) {
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

    pub fn renderer<'b>(
        &'b mut self,
        target: &'b RenderTarget
    ) -> Renderer<'b> {
        Renderer::new(target, self)
    }

    pub fn renderer_with_camera<'b>(
        &'b mut self,
        target: &'b RenderTarget,
        camera: &'b CameraBuffer,
    ) -> Renderer<'b> {
        let mut renderer = self.renderer(target);
        renderer.use_camera(&camera);
        return renderer;
    }

    pub fn world_renderer_with_camera<'b>(&'b mut self, camera: &'b CameraBuffer) -> Renderer<'b> {
        return self.renderer_with_camera(&self.defaults.target, camera);
    }

    pub fn world_renderer<'b>(&'b mut self) -> Renderer<'b> {
        return self.world_renderer_with_camera(&self.defaults.world_camera);
    }

    #[cfg(feature = "text")]
    pub fn render_text(&mut self, target: &RenderTarget, gpu: &Gpu, descriptor: TextDescriptor) {
        let target_size = target.size();
        let mut staging_belt = wgpu::util::StagingBelt::new(1024);
        if let Some(color) = descriptor.clear_color {
            self.clear(target, color);
        }

        for section in descriptor.sections {
            descriptor
                .font
                .brush
                .queue(section.to_glyph_section(descriptor.resolution));
        }

        descriptor
            .font
            .brush
            .draw_queued(
                &gpu.device,
                &mut staging_belt,
                &mut self.inner,
                target.view(),
                (descriptor.resolution * target_size.x as f32) as u32,
                (descriptor.resolution * target_size.y as f32) as u32,
            )
            .expect("Draw queued");

        staging_belt.finish();
    }

    pub fn copy_to_target(&mut self, defaults: &GpuDefaults, src: &Sprite, target: &RenderTarget) {
        let mut renderer = Renderer::new(target, self);
        renderer.use_camera(&defaults.relative_camera);
        renderer.use_instances(&defaults.single_centered_instance);
        renderer.use_shader(&defaults.sprite);
        renderer.use_model(defaults.relative_camera.model());
        renderer.use_sprite(src, 1);
        renderer.draw(0);
    }

    pub fn submit(self, gpu: &Gpu) {
        gpu.queue.submit(Some(self.inner.finish()));
    }
}
