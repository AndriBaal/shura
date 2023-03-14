use std::ops::Deref;

use crate::{
    Gpu, GpuDefaults, RenderConfig, RenderEncoder, Sprite, Vector,
};
macro_rules! Where {
    (
    $a:lifetime >= $b:lifetime $(,)?
) => {
        &$b & $a()
    };
}

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

    pub fn computed<'caller>(
        gpu: &Gpu,
        defaults: &GpuDefaults,
        texture_size: Vector<u32>,
        compute: impl for<'any> Fn(&mut RenderEncoder, RenderConfig<'any>, [Where!('caller >= 'any); 0]),
    ) -> Self {
        let target = RenderTarget::new(gpu, texture_size);
        target.draw(gpu, defaults, compute);
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

    // pub fn draw(
    //     &self,
    //     gpu: &Gpu,
    //     defaults: &GpuDefaults,
    //     compute: impl Fn(&mut RenderEncoder, RenderConfig),
    // ) {
    //     let mut encoder = RenderEncoder::new(gpu);
    //     let config = RenderConfig {
    //         camera: RenderCamera::RelativeCamera,
    //         instances: RenderInstances::SingleInstance,
    //         target: &self,
    //         gpu: &gpu,
    //         defaults: &defaults,
    //         smaa: true
    //     };
    //     compute(&mut encoder, config);
    //     gpu.queue.submit(std::iter::once(encoder.encoder.finish()));
    // }

    // pub fn draw1<'test>(
    //     &'test self,
    //     gpu: &'test Gpu,
    //     defaults: &'test GpuDefaults,
    //     compute: impl Fn(&'test mut RenderEncoder, RenderConfig<'test>),
    // ) {
    //     let mut encoder = RenderEncoder::new(gpu);
    //     let config = RenderConfig {
    //         camera: RenderCamera::RelativeCamera,
    //         instances: RenderInstances::SingleInstance,
    //         target: &self,
    //         gpu: &gpu,
    //         defaults: &defaults,
    //         smaa: true
    //     };
    //     compute(&mut encoder, config);
    //     gpu.queue.submit(std::iter::once(encoder.encoder.finish()));
    // }

    pub fn draw<'caller>(
        &self,
        gpu: &Gpu,
        defaults: &GpuDefaults,
        compute: impl for<'any> Fn(&mut RenderEncoder, RenderConfig<'any>, [Where!('caller >= 'any); 0]),
    ) {
        let mut encoder = RenderEncoder::new(gpu);
        let config = RenderConfig {
            camera: &defaults.relative_camera,
            instances: &defaults.single_centered_instance,
            target: &self,
            gpu: &gpu,
            defaults: &defaults,
            smaa: true
        };
        compute(&mut encoder, config, []);
        gpu.queue.submit(std::iter::once(encoder.encoder.finish()));
    }
}

impl Into<Sprite> for RenderTarget {
    fn into(self) -> Sprite {
        return self.target;
    }
}

impl Deref for RenderTarget {
    type Target = Sprite;

    fn deref(&self) -> &Sprite {
        self.sprite()
    }
}
