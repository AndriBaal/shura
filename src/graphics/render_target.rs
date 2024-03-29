use std::{ops::Deref, sync::Arc};

use downcast_rs::{impl_downcast, Downcast};
use winit::{
    event::{Event, StartCause},
    window::Window,
};

use crate::{
    graphics::{Camera2D, Color, Gpu, RenderEncoder, Sprite, SpriteBuilder},
    math::Vector2,
};

pub trait RenderTarget: Downcast {
    fn msaa(&self) -> Option<&wgpu::TextureView>;
    fn view(&self) -> &wgpu::TextureView;
    fn texture(&self) -> &wgpu::Texture;
    fn size(&self) -> Vector2<u32> {
        Vector2::new(self.texture().width(), self.texture().height())
    }
    fn attachment(&self, clear: Option<Color>) -> wgpu::RenderPassColorAttachment {
        wgpu::RenderPassColorAttachment {
            view: if let Some(msaa) = self.msaa() {
                msaa
            } else {
                self.view()
            },
            resolve_target: if self.msaa().is_some() {
                Some(self.view())
            } else {
                None
            },
            ops: wgpu::Operations {
                load: if let Some(clear_color) = clear {
                    wgpu::LoadOp::Clear(clear_color.into())
                } else {
                    wgpu::LoadOp::Load
                },
                store: wgpu::StoreOp::Store,
            },
        }
    }

    fn as_copy(&self) -> wgpu::ImageCopyTexture {
        self.texture().as_image_copy()
    }
}
impl_downcast!(RenderTarget);

pub struct Surface {
    target_msaa: Option<wgpu::Texture>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl Surface {
    pub(crate) fn new() -> Self {
        Self {
            target_msaa: None,
            surface: None,
            config: None,
        }
    }

    /// Called after the instance is created, but before we request an adapter.
    ///
    /// On wasm, we need to create the surface here, as the WebGL backend needs
    /// a surface (and hence a canvas) to be present to create the adapter.
    ///
    /// We cannot unconditionally create a surface here, as Android requires
    /// us to wait until we recieve the `Resumed` event to do so.
    pub(crate) fn pre_adapter(&mut self, instance: &wgpu::Instance, window: Arc<Window>) {
        if cfg!(target_arch = "wasm32") {
            self.init(instance, window);
        }
    }

    fn init(&mut self, instance: &wgpu::Instance, window: Arc<Window>) {
        self.surface = Some(instance.create_surface(window).unwrap());
    }

    pub(crate) fn update_msaa(&mut self, gpu: &Gpu, size: Vector2<u32>) {
        if gpu.samples() != 1 && (size != self.size() || self.target_msaa.is_none()) {
            self.target_msaa = Some(SpriteRenderTarget::create_msaa(gpu, size));
        }
    }

    /// Check if the event is the start condition for the surface.
    pub(crate) fn start_condition(e: &Event<()>) -> bool {
        match e {
            // On all other platforms, we can create the surface immediately.
            Event::NewEvents(StartCause::Init) => !cfg!(target_os = "android"),
            // On android we need to wait for a resumed event to create the surface.
            Event::Resumed => cfg!(target_os = "android"),
            _ => false,
        }
    }

    pub(crate) fn resume(&mut self, gpu: &Gpu, window: Arc<Window>) {
        // We didn't create the surface in pre_adapter, so we need to do so now.
        if !cfg!(target_arch = "wasm32") {
            self.init(&gpu.instance, window.clone());
        }

        // From here on, self.surface should be Some.
        let surface = self.surface.as_ref().unwrap();

        // Window size is only actually valid after we enter the event loop.
        let window_size = window.inner_size();
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);
        let mut config = surface
            .get_default_config(&gpu.adapter, width, height)
            .expect("Surface isn't supported by the adapter.");
        if !cfg!(target_arch = "wasm32") {
            config.usage |= wgpu::TextureUsages::COPY_SRC;
        }

        surface.configure(&gpu.device, &config);
        self.config = Some(config);
    }

    /// Resize the surface, making sure to not resize to zero.
    pub(crate) fn resize(&mut self, gpu: &Gpu, size: Vector2<u32>) {
        #[cfg(feature = "log")]
        log::info!("Surface resize {size:?}");
        self.update_msaa(gpu, size);

        let config = self.config.as_mut().unwrap();
        config.width = size.x.max(1);
        config.height = size.y.max(1);
        let surface = self.surface.as_ref().unwrap();
        surface.configure(&gpu.device, config);
    }

    /// Acquire the next surface texture.
    pub(crate) fn start_frame(&mut self, gpu: &Gpu) -> SurfaceRenderTarget {
        let surface = self.surface.as_ref().unwrap();

        let surface_texture = match surface.get_current_texture() {
            Ok(frame) => frame,
            // If we timed out, just try again
            Err(wgpu::SurfaceError::Timeout) => surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture!"),
            Err(
                // If the surface is outdated, or was lost, reconfigure it.
                wgpu::SurfaceError::Outdated
                | wgpu::SurfaceError::Lost
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try
                | wgpu::SurfaceError::OutOfMemory,
            ) => {
                surface.configure(&gpu.device, self.config());
                surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        };

        return SurfaceRenderTarget {
            target_view: surface_texture.texture.create_view(&Default::default()),
            msaa_view: self
                .target_msaa
                .as_ref()
                .map(|msaa| msaa.create_view(&Default::default())),
            surface_texture,
        };
    }

    /// On suspend on android, we drop the surface, as it's no longer valid.
    ///
    /// A suspend event is always followed by at least one resume event.
    pub(crate) fn suspend(&mut self) {
        // if cfg!(target_os = "android") {
        //     self.surface = None;
        // }
    }

    pub(crate) fn apply_vsync(&mut self, gpu: &Gpu, vsync: bool) {
        let config = self.config.as_mut().unwrap();
        let surface = self.surface.as_ref().unwrap();
        let new_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        config.present_mode = new_mode;
        surface.configure(&gpu.device, config);
    }

    pub fn size(&self) -> Vector2<u32> {
        let config = self.config();
        Vector2::new(config.width, config.height)
    }

    pub fn surface(&self) -> Option<&wgpu::Surface> {
        self.surface.as_ref()
    }

    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        self.config.as_ref().unwrap()
    }
}

pub struct SurfaceRenderTarget {
    surface_texture: wgpu::SurfaceTexture,
    target_view: wgpu::TextureView,
    msaa_view: Option<wgpu::TextureView>,
}

impl SurfaceRenderTarget {
    pub(crate) fn finish(self) {
        self.surface_texture.present();
    }
}

impl RenderTarget for SurfaceRenderTarget {
    fn view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    fn texture(&self) -> &wgpu::Texture {
        &self.surface_texture.texture
    }

    fn msaa(&self) -> Option<&wgpu::TextureView> {
        self.msaa_view.as_ref()
    }
}

impl SurfaceRenderTarget {}

impl RenderTarget for SpriteRenderTarget {
    fn view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    fn msaa(&self) -> Option<&wgpu::TextureView> {
        self.target_msaa.as_ref()
    }

    fn texture(&self) -> &wgpu::Texture {
        self.sprite().texture()
    }
}

#[derive(Debug)]
pub struct SpriteRenderTarget {
    target_msaa: Option<wgpu::TextureView>,
    target_view: wgpu::TextureView,
    target: Sprite,
}

impl SpriteRenderTarget {
    pub fn new(gpu: &Gpu, size: Vector2<u32>) -> Self {
        Self::custom(gpu, SpriteBuilder::empty(size).format(gpu.format()))
    }

    pub fn custom<D: Deref<Target = [u8]>>(gpu: &Gpu, sprite: SpriteBuilder<D>) -> Self {
        let size = sprite.size;
        let target = Sprite::new(gpu, sprite.format(gpu.format()));
        let target_view = target
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target_msaa = if gpu.samples() == 1 {
            None
        } else {
            Some(SpriteRenderTarget::create_msaa(gpu, size).create_view(&Default::default()))
        };

        Self {
            target_msaa,
            target,
            target_view,
        }
    }

    pub fn computed<D: Deref<Target = [u8]>>(
        gpu: &Gpu,
        sprite: SpriteBuilder<D>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> Self {
        let target = SpriteRenderTarget::custom(gpu, sprite);
        target.draw(gpu, compute);
        target
    }

    pub fn create_msaa(gpu: &Gpu, size: Vector2<u32>) -> wgpu::Texture {
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: gpu.samples(),
            dimension: wgpu::TextureDimension::D2,
            format: gpu.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        gpu.device.create_texture(multisampled_frame_descriptor)
    }

    pub fn resize(&mut self, gpu: &Gpu, size: Vector2<u32>) {
        if self.size() != size {
            *self = Self::new(gpu, size);
        }
    }

    pub fn sprite(&self) -> &Sprite {
        &self.target
    }

    pub fn draw(&self, gpu: &Gpu, compute: impl FnOnce(&mut RenderEncoder)) {
        let default_assets = gpu.default_assets();
        let mut encoder = RenderEncoder::new(gpu, self, &default_assets);
        compute(&mut encoder);
    }

    pub fn compute_target_size(
        mesh_half_extents: Vector2<f32>,
        camera: &Camera2D,
        window_size: Vector2<u32>,
    ) -> Vector2<u32> {
        let camera_fov = camera.fov() * 2.0;
        let size = mesh_half_extents * 2.0;
        Vector2::new(
            (size.x / camera_fov.x * window_size.x as f32).ceil() as u32,
            (size.y / camera_fov.y * window_size.y as f32).ceil() as u32,
        )
    }
}

impl From<SpriteRenderTarget> for Sprite {
    fn from(color: SpriteRenderTarget) -> Self {
        color.target
    }
}
