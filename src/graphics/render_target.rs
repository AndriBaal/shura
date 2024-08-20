use downcast_rs::{impl_downcast, Downcast};
use std::ops::Deref;

use crate::{
    graphics::{Camera2D, Color, Gpu, RenderEncoder, Sprite, SpriteBuilder},
    math::Vector2,
};

use super::{Uniform, GLOBAL_ASSETS, GLOBAL_GPU};

pub trait RenderTarget: Downcast + Send + Sync {
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

pub struct SurfaceRenderTarget {
    pub surface_texture: wgpu::SurfaceTexture,
    pub target_view: wgpu::TextureView,
    pub msaa_view: Option<wgpu::TextureView>,
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
        sprite: SpriteBuilder<D>,
        compute: impl FnMut(&mut RenderEncoder),
    ) -> Self {
        let gpu = GLOBAL_GPU.get().unwrap();
        let target = SpriteRenderTarget::custom(gpu, sprite);
        target.draw(compute);
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

    pub fn draw(&self, compute: impl FnOnce(&mut RenderEncoder)) {
        let assets = GLOBAL_ASSETS.get().unwrap();
        let gpu = GLOBAL_GPU.get().unwrap();
        let default_assets = assets.default_assets();
        let mut encoder = RenderEncoder::new(gpu, assets, &default_assets, self);
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

impl Uniform for SpriteRenderTarget {
    fn bind_group(&self) -> &wgpu::BindGroup {
        self.sprite().bind_group()
    }
}

impl From<SpriteRenderTarget> for Sprite {
    fn from(color: SpriteRenderTarget) -> Self {
        color.target
    }
}
