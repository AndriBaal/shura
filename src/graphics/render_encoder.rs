use crate::{Color, Gpu, GpuDefaults, RenderTarget, Renderer};

/// Encoder of [Renderers](crate::Renderer) and utilities to copy, clear and render text onto [RenderTargets](crate::RenderTarget)
pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub defaults: &'a GpuDefaults,
    pub gpu: &'a Gpu,
}

impl<'a> RenderEncoder<'a> {
    pub fn new(gpu: &'a Gpu, defaults: &'a GpuDefaults) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        Self {
            inner: encoder,
            defaults,
            gpu,
        }
    }

    pub fn clear(&mut self, target: &dyn RenderTarget, color: Color) {
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
        target: &'b dyn RenderTarget,
        clear: Option<Color>,
        msaa: bool,
    ) -> Renderer<'b> {
        Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
            target,
            msaa,
            clear,
        )
    }

    pub fn copy_target(&mut self, src: &dyn RenderTarget, target: &dyn RenderTarget) {
        assert_eq!(src.size(), target.size());
        let size = wgpu::Extent3d {
            width: src.size().x,
            height: src.size().y,
            depth_or_array_layers: 1,
        };
        self.inner
            .copy_texture_to_texture(src.as_copy(), target.as_copy(), size);
    }

    pub fn finish(self) -> wgpu::CommandBuffer {
        self.inner.finish()
    }

    pub fn submit(self, gpu: &Gpu) -> wgpu::SubmissionIndex {
        gpu.submit(self)
    }
}
