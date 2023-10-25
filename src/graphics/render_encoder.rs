use crate::{
    Color, DefaultResources, Gpu, RenderTarget, Renderer, SpriteRenderTarget, WorldCamera,
};

/// Encoder of [Renderers](crate::Renderer) and utilities to copy, clear and render text onto [RenderTargets](crate::RenderTarget)
pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub defaults: &'a DefaultResources,
    pub gpu: &'a Gpu,
    pub world_camera: &'a WorldCamera,
}

impl <'a>Clone for RenderEncoder<'a> {
    fn clone(&self) -> Self {
        Self::new(self.gpu, self.defaults, self.world_camera)
    }
}

impl<'a> RenderEncoder<'a> {
    pub fn new(
        gpu: &'a Gpu,
        defaults: &'a DefaultResources,
        world_camera: &'a WorldCamera,
    ) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        Self {
            inner: encoder,
            defaults,
            gpu,
            world_camera,
        }
    }

    pub fn render<'b>(
        &'b mut self,
        clear: Option<Color>,
        render: impl FnOnce(&mut Renderer<'b>)
    ) {
        let mut renderer = Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
            self.world_camera,
            self.defaults.default_target(),
            clear,
        );
        (render)(&mut renderer);
    }

    pub fn render_to(
        &mut self,
        target: &dyn RenderTarget,
        clear: Option<Color>,
        render: impl FnOnce(&mut Renderer)
    ) {
        let mut renderer = Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
            self.world_camera,
            target,
            clear,
        );
        (render)(&mut renderer);
    }

    pub fn renderer<'b>(
        &'b mut self,
        clear: Option<Color>,
    ) -> Renderer<'b> {
        Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
            self.world_camera,
            self.defaults.default_target(),
            clear,
        )
    }

    pub fn renderer_to<'b>(
        &'b mut self,
        target: &'b dyn RenderTarget,
        clear: Option<Color>,
    ) -> Renderer<'b> {
        Renderer::new(
            &mut self.inner,
            self.defaults,
            self.gpu,
            self.world_camera,
            target,
            clear,
        )
    }

    pub fn deep_copy_target(&mut self, src: &dyn RenderTarget, target: &dyn RenderTarget) {
        assert_eq!(src.size(), target.size());
        let size = wgpu::Extent3d {
            width: src.size().x,
            height: src.size().y,
            depth_or_array_layers: 1,
        };
        self.inner
            .copy_texture_to_texture(src.as_copy(), target.as_copy(), size);
    }

    pub fn copy_target(&mut self, src: &SpriteRenderTarget, target: &dyn RenderTarget) {
        let mut renderer = self.renderer_to(target, None);
        renderer.render_sprite(
            0..1,
            &renderer.defaults.centered_instance,
            &renderer.defaults.unit_camera,
            renderer.defaults.unit_model(),
            src.sprite(),
        );
    }

    pub fn finish(self) -> wgpu::CommandBuffer {
        self.inner.finish()
    }

    pub fn submit(self, gpu: &Gpu) -> wgpu::SubmissionIndex {
        gpu.submit(self)
    }
}
