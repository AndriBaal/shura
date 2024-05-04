use crate::{graphics::Gpu, math::Vector2};

#[derive(Debug)]
pub struct DepthBuffer {
    view: wgpu::TextureView,
    size: Vector2<u32>,
    format: wgpu::TextureFormat,
}

impl DepthBuffer {
    pub const DEPTH_FORMAT_3D: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn new(gpu: &Gpu, size: Vector2<u32>, format: wgpu::TextureFormat) -> Self {
        let extend = wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: extend,
            mip_level_count: 1,
            sample_count: gpu.samples(),
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            view_formats: &[],
        };
        let texture = gpu.device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self { view, size, format }
    }

    pub fn resize(&mut self, gpu: &Gpu, size: Vector2<u32>) {
        if self.size != size {
            *self = Self::new(gpu, size, self.format);
        }
    }

    pub fn size(&self) -> Vector2<u32> {
        self.size
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
