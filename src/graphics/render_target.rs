use std::ops::Deref;

use downcast_rs::{impl_downcast, Downcast};
use wgpu::SurfaceTexture;

use crate::{
    Camera, Color, Gpu, DefaultResources, RenderEncoder, Sprite, SpriteBuilder, Vector, WorldCamera,
};

pub trait RenderTarget: Downcast {
    fn msaa(&self) -> Option<&wgpu::TextureView>;
    fn view(&self) -> &wgpu::TextureView;
    fn as_copy(&self) -> wgpu::ImageCopyTexture;
    fn size(&self) -> Vector<u32>;
    fn texture(&self) -> &wgpu::Texture;
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
                store: true,
            },
        }
    }
}
impl_downcast!(RenderTarget);

pub struct SurfaceRenderTarget {
    surface: Option<SurfaceTexture>,
    target_view: Option<wgpu::TextureView>,
    target_msaa: Option<wgpu::TextureView>,
    size: Vector<u32>,
}

impl SurfaceRenderTarget {
    pub fn new(gpu: &Gpu, size: Vector<u32>) -> Self {
        let target_msaa = if gpu.sample_count() == 1 {
            None
        } else {
            Some(SpriteRenderTarget::create_msaa(gpu, size))
        };
        Self {
            surface: None,
            target_view: None,
            target_msaa,
            size,
        }
    }

    pub(crate) fn resize(&mut self, gpu: &Gpu, new_size: Vector<u32>) {
        if gpu.sample_count() != 1 && new_size != self.size {
            self.target_msaa = Some(SpriteRenderTarget::create_msaa(gpu, new_size));
        }
        self.size = new_size;
    }

    pub(crate) fn start_frame(&mut self, gpu: &Gpu) -> Result<(), wgpu::SurfaceError> {
        let surface = gpu.surface.lock().unwrap();
        let config = gpu.config.lock().unwrap();
        let surface = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(_) => {
                surface.configure(&gpu.device, &config);
                surface.get_current_texture()?
            }
        };
        self.target_view = Some(surface.texture.create_view(&Default::default()));
        self.surface = Some(surface);
        return Ok(());
    }

    pub(crate) fn finish_frame(&mut self) {
        self.target_view.take();
        let surface = self.surface.take().unwrap();
        surface.present();
    }
}

impl RenderTarget for SurfaceRenderTarget {
    fn view(&self) -> &wgpu::TextureView {
        self.target_view
            .as_ref()
            .expect("Surface texture only available while rendering!")
    }

    fn as_copy(&self) -> wgpu::ImageCopyTexture {
        self.surface
            .as_ref()
            .expect("Surface texture only available while rendering!")
            .texture
            .as_image_copy()
    }

    fn texture(&self) -> &wgpu::Texture {
        &self
            .surface
            .as_ref()
            .expect("Surface texture only available while rendering!")
            .texture
    }

    fn msaa(&self) -> Option<&wgpu::TextureView> {
        self.target_msaa.as_ref()
    }

    fn size(&self) -> Vector<u32> {
        self.size
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

    fn size(&self) -> Vector<u32> {
        self.target.size()
    }

    fn as_copy(&self) -> wgpu::ImageCopyTexture {
        self.sprite().texture().as_image_copy()
    }

    fn texture(&self) -> &wgpu::Texture {
        self.sprite().texture()
    }
}

/// Texture to render onto with a [RenderEncoder]
pub struct SpriteRenderTarget {
    target_msaa: Option<wgpu::TextureView>,
    target_view: wgpu::TextureView,
    target: Sprite,
}

impl SpriteRenderTarget {
    pub fn new(gpu: &Gpu, size: Vector<u32>) -> Self {
        Self::custom(gpu, SpriteBuilder::empty(size).format(gpu.format()))
    }

    pub fn custom<D: Deref<Target = [u8]>>(gpu: &Gpu, sprite: SpriteBuilder<D>) -> Self {
        let size = sprite.size;
        let target = Sprite::new(gpu, sprite.format(gpu.format()));
        let target_view = target
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target_msaa = if gpu.sample_count() == 1 {
            None
        } else {
            Some(SpriteRenderTarget::create_msaa(gpu, size))
        };

        return Self {
            target_msaa,
            target,
            target_view,
        };
    }

    pub fn computed<D: Deref<Target = [u8]>>(
        gpu: &Gpu,
        defaults: &DefaultResources,
        world_camera: &WorldCamera,
        sprite: SpriteBuilder<D>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> Self {
        let target = SpriteRenderTarget::custom(gpu, sprite);
        target.draw(gpu, defaults, world_camera, compute);
        return target;
    }

    pub fn create_msaa(gpu: &Gpu, size: Vector<u32>) -> wgpu::TextureView {
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: gpu.sample_count(),
            dimension: wgpu::TextureDimension::D2,
            format: gpu.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        gpu.device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn resize(&mut self, gpu: &Gpu, size: Vector<u32>) {
        if self.size() != size {
            *self = Self::new(gpu, size);
        }
    }

    pub fn sprite(&self) -> &Sprite {
        &self.target
    }

    pub fn draw(
        &self,
        gpu: &Gpu,
        defaults: &DefaultResources,
        world_camera: &WorldCamera,
        compute: impl FnOnce(&mut RenderEncoder),
    ) {
        let mut encoder = RenderEncoder::new(gpu, defaults, world_camera);
        compute(&mut encoder);
        encoder.submit(gpu);
    }

    pub fn compute_target_size(
        model_half_extents: Vector<f32>,
        camera: &Camera,
        window_size: Vector<u32>,
    ) -> Vector<u32> {
        let camera_fov = camera.fov() * 2.0;
        let size = model_half_extents * 2.0;
        return Vector::new(
            (size.x / camera_fov.x * window_size.x as f32).ceil() as u32,
            (size.y / camera_fov.y * window_size.y as f32).ceil() as u32,
        );
    }

    // pub fn validate_webgl_size(mut size: Vector<u32>) -> Vector<u32> {
    //     use log::warn;
    //     const MAX_WEBGL_TEXTURE_SIZE: u32 = 2048;
    //     if size.x > MAX_WEBGL_TEXTURE_SIZE {
    //         size.x = MAX_WEBGL_TEXTURE_SIZE;
    //         warn!("Auto scaling down to x {MAX_WEBGL_TEXTURE_SIZE} because the maximum WebGL texturesize has been surpassed!");
    //     }
    //     if size.x > MAX_WEBGL_TEXTURE_SIZE {
    //         size.x = MAX_WEBGL_TEXTURE_SIZE;
    //         warn!("Auto scaling down to x {MAX_WEBGL_TEXTURE_SIZE} because the maximum WebGL texturesize has been surpassed!");
    //     }
    //     return size;
    // }
}

impl Into<Sprite> for SpriteRenderTarget {
    fn into(self) -> Sprite {
        return self.target;
    }
}
