use crate::graphics::{
    AssetManager, Color, DefaultAssets, DepthBuffer, Gpu, RenderTarget, Renderer,
    SpriteRenderTarget,
};

pub struct RenderEncoder<'a> {
    pub inner: wgpu::CommandEncoder,
    pub assets: &'a AssetManager,
    pub default_assets: &'a DefaultAssets,
    pub gpu: &'a Gpu,
    pub default_target: &'a dyn RenderTarget,
}

impl<'a> Clone for RenderEncoder<'a> {
    fn clone(&self) -> Self {
        Self::new(
            self.gpu,
            self.assets,
            self.default_assets,
            self.default_target,
        )
    }
}

impl<'a> RenderEncoder<'a> {
    pub fn new(
        gpu: &'a Gpu,
        assets: &'a AssetManager,
        default_assets: &'a DefaultAssets,
        default_target: &'a dyn RenderTarget,
    ) -> Self {
        let encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        Self {
            inner: encoder,
            assets,
            default_assets,
            default_target,
            gpu,
        }
    }

    pub fn render2d<'b>(
        &'b mut self,
        clear: Option<Color>,
        render: impl FnOnce(&mut Renderer<'b>),
    ) {
        let mut renderer = self.renderer(self.default_target, clear, None);
        (render)(&mut renderer);
    }

    pub fn render2d_to<'b>(
        &'b mut self,
        clear: Option<Color>,
        target: &'b dyn RenderTarget,
        render: impl FnOnce(&mut Renderer<'b>),
    ) {
        let mut renderer = self.renderer(target, clear, None);
        (render)(&mut renderer);
    }

    pub fn render3d<'b>(
        &'b mut self,
        clear: Option<Color>,
        render: impl FnOnce(&mut Renderer<'b>),
    ) {
        let mut renderer = self.renderer(
            self.default_target,
            clear,
            Some(&self.default_assets.depth_buffer),
        );

        (render)(&mut renderer);
    }

    pub fn render3d_to<'b>(
        &'b mut self,
        target: &'b dyn RenderTarget,
        clear: Option<Color>,
        depth: Option<&'b DepthBuffer>,
        render: impl FnOnce(&mut Renderer<'b>),
    ) {
        let mut renderer = self.renderer(target, clear, depth);
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
            self.assets,
            self.default_assets,
            self.gpu,
            target,
            clear,
            depth,
        )
    }

    pub fn renderer2d<'b>(
        &'b mut self,
        clear: Option<Color>,
    ) -> Renderer<'b> {
        self.renderer2d_to(self.default_target, clear)
    }


    pub fn renderer2d_to<'b>(
        &'b mut self,
        target: &'b dyn RenderTarget,
        clear: Option<Color>,
    ) -> Renderer<'b> {
        self.renderer(target, clear, None)
    }


    pub fn copy_target(&mut self, src: &dyn RenderTarget, target: &dyn RenderTarget) {
        // if src
        //     .texture()
        //     .usage()
        //     .contains(wgpu::TextureUsages::COPY_SRC)
        //     && target
        //         .texture()
        //         .usage()
        //         .contains(wgpu::TextureUsages::COPY_DST)
        // {
        //     let size = wgpu::Extent3d {
        //         width: src.size().x,
        //         height: src.size().y,
        //         depth_or_array_layers: 1,
        //     };
        //     self.inner
        //         .copy_texture_to_texture(src.as_copy(), target.as_copy(), size);
        // } else {
        let src = src
            .downcast_ref::<SpriteRenderTarget>()
            .expect("Cannot copy this texture!");
        let mut renderer = self.renderer(target, None, None);
        renderer.draw_sprite_mesh(
            &renderer.default_assets.unit_camera.0,
            &renderer.default_assets.sprite_mesh,
            src.sprite(),
        );
    }

    pub fn finish_get(self) -> wgpu::CommandBuffer {
        self.inner.finish()
    }

    pub fn finish(self) {
        self.gpu.command_buffers.lock().push(self.finish_get())
    }
}
