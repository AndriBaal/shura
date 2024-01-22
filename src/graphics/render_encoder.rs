use crate::graphics::{
    Color, DefaultResources, DepthBuffer, Gpu, RenderTarget, Renderer, SpriteRenderTarget,
};

pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub default_resources: &'a DefaultResources,
    pub gpu: &'a Gpu,
    pub default_target: &'a dyn RenderTarget,
}

impl<'a> Clone for RenderEncoder<'a> {
    fn clone(&self) -> Self {
        Self::new(self.gpu, self.default_target, self.default_resources)
    }
}

impl<'a> RenderEncoder<'a> {
    pub fn new(
        gpu: &'a Gpu,
        default_target: &'a dyn RenderTarget,
        default_resources: &'a DefaultResources,
    ) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        Self {
            inner: encoder,
            default_resources,
            default_target,
            gpu,
        }
    }

    pub fn render2d<'b>(
        &'b mut self,
        clear: Option<Color>,
        render: impl FnOnce(&mut Renderer<'b>),
    ) {
        let mut renderer = Renderer::new(
            &mut self.inner,
            self.default_resources,
            self.gpu,
            self.default_target,
            clear,
            None,
        );
        (render)(&mut renderer);
    }

    pub fn render3d<'b>(
        &'b mut self,
        clear: Option<Color>,
        render: impl FnOnce(&mut Renderer<'b>),
    ) {
        let mut renderer = Renderer::new(
            &mut self.inner,
            self.default_resources,
            self.gpu,
            self.default_target,
            clear,
            Some(&self.default_resources.depth_buffer),
        );
        (render)(&mut renderer);
    }

    pub fn renderer<'b>(
        &'b mut self,
        target: &'b dyn RenderTarget,
        clear: Option<Color>,
        depth: Option<&'b DepthBuffer>,
    ) -> Renderer<'b> {
        Renderer::new(
            &mut self.inner,
            self.default_resources,
            self.gpu,
            target,
            clear,
            depth,
        )
    }

    pub fn copy_target(&mut self, src: &dyn RenderTarget, target: &dyn RenderTarget) {
        if src
            .texture()
            .usage()
            .contains(wgpu::TextureUsages::COPY_SRC)
            && target
                .texture()
                .usage()
                .contains(wgpu::TextureUsages::COPY_DST)
        {
            let size = wgpu::Extent3d {
                width: src.size().x,
                height: src.size().y,
                depth_or_array_layers: 1,
            };
            self.inner
                .copy_texture_to_texture(src.as_copy(), target.as_copy(), size);
        } else {
            let src = src
                .downcast_ref::<SpriteRenderTarget>()
                .expect("Cannot copy this texture!");
            let mut renderer = self.renderer(target, None, None);
            renderer.render_sprite(
                0..1,
                &renderer.default_resources.centered_instance,
                &renderer.default_resources.unit_camera.0,
                renderer.default_resources.unit_mesh(),
                src.sprite(),
            );
        }
    }

    pub fn finish_get(self) -> wgpu::CommandBuffer {
        self.inner.finish()
    }

    pub fn finish(self) {
        self.gpu
            .command_buffers
            .lock()
            .unwrap()
            .push(self.finish_get())
    }
}
