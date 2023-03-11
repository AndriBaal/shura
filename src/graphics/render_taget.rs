use crate::{
    Gpu, GpuDefaults, RenderCamera, RenderConfig, RenderEncoder, RenderInstances, Sprite, Vector,
};

pub struct RenderTarget {
    target_msaa: wgpu::TextureView,
    target_view: wgpu::TextureView,
    target: Sprite,
}

impl RenderTarget {
    pub fn new(gpu: &Gpu, size: Vector<u32>) -> Self {
        let target = Sprite::empty(gpu, size);
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

    pub fn computed(
        gpu: &Gpu,
        defaults: &GpuDefaults,
        instances: RenderInstances,
        camera: RenderCamera,
        texture_size: Vector<u32>,
        compute: impl Fn(&mut RenderEncoder, RenderConfig),
    ) -> Self {
        let target = RenderTarget::new(gpu, texture_size);
        target.draw(gpu, defaults, instances, camera, compute);
        return target;
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

    pub fn sprite(&self) -> &Sprite {
        &self.target
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    pub fn msaa(&self) -> &wgpu::TextureView {
        &self.target_msaa
    }

    pub fn draw(
        &self,
        gpu: &Gpu,
        defaults: &GpuDefaults,
        instances: RenderInstances,
        camera: RenderCamera,
        compute: impl Fn(&mut RenderEncoder, RenderConfig),
    ) {
        let mut encoder = RenderEncoder::new(gpu);
        let config = RenderConfig {
            camera,
            instances,
            target: &self,
            gpu: &gpu,
            defaults: &defaults,
        };
        compute(&mut encoder, config);
        gpu.queue.submit(std::iter::once(encoder.encoder.finish()));
    }
}

impl Into<Sprite> for RenderTarget {
    fn into(self) -> Sprite {
        return self.target;
    }
}
