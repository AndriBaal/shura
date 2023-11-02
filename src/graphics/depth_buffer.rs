use crate::{Gpu, Vector2};

pub struct DepthBuffer {
    view: wgpu::TextureView,
    size: Vector2<u32>,
}

impl DepthBuffer {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn depth_state() -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: DepthBuffer::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }

    pub fn new(gpu: &Gpu, size: Vector2<u32>) -> Self {
        let extend = wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: extend,
            mip_level_count: 1,
            sample_count: gpu.sample_count(),
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = gpu.device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        return Self { view, size };
    }

    pub fn resize(&mut self, gpu: &Gpu, size: Vector2<u32>) {
        if self.size != size {
            *self = Self::new(gpu, size);
        }
    }

    pub fn size(&self) -> Vector2<u32> {
        self.size
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
