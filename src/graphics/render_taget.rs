use crate::{Gpu, Sprite, Vector};

pub struct RenderTarget {
    target_msaa: wgpu::TextureView,
    target_view: wgpu::TextureView,
    target: Sprite,
}

impl RenderTarget {
    pub fn new(gpu: &Gpu, size: Vector<u32>) -> Self {
        let target = Sprite::empty(gpu, size);
        let sample_count = gpu.base.sample_count;
        let format = gpu.config.format;
        let target_view = target
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target_msaa =
            Self::create_msaa(&gpu.device, gpu.config.format, gpu.base.sample_count, size);

        return Self {
            target_msaa,
            target,
            target_view,
        };
    }

    pub fn create_msaa(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        sample_count: u32,
        size: Vector<u32>,
    ) -> wgpu::TextureView {
        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };

        device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn target(&self) -> &Sprite {
        &self.target
    }

    pub fn target_view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    pub fn target_msaa(&self) -> &wgpu::TextureView {
        &self.target_msaa
    }
}

impl Into<Sprite> for RenderTarget {
    fn into(self) -> Sprite {
        return self.target;
    }
}
