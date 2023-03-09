use crate::{
    CameraBuffer, Color, Gpu, GpuDefaults, InstanceBuffer, Instances, Renderer, Sprite, Vector,
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

    pub fn computed<'caller, F>(
        gpu: &Gpu,
        defaults: &GpuDefaults,
        instances: &InstanceBuffer,
        camera: &CameraBuffer,
        texture_size: Vector<u32>,
        clear_color: Option<Color>,
        compute: F,
    ) -> Self
    where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        let target = RenderTarget::new(gpu, texture_size);
        target.draw(gpu, defaults, instances, camera, clear_color, compute);
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

    pub fn target(&self) -> &Sprite {
        &self.target
    }

    pub fn target_view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    pub fn target_msaa(&self) -> &wgpu::TextureView {
        &self.target_msaa
    }

    pub fn draw<'caller, F>(
        &self,
        gpu: &Gpu,
        defaults: &GpuDefaults,
        instance_buffer: &InstanceBuffer,
        camera: &CameraBuffer,
        clear_color: Option<Color>,
        compute: F,
    ) where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        let mut encoder = gpu.encoder();
        {
            let mut renderer =
                Renderer::new(&mut encoder, gpu, defaults, self, camera, clear_color);
            renderer.use_uniform(&camera.uniform(), 0);
            renderer.set_instance_buffer(instance_buffer);
            compute(&mut renderer, instance_buffer.instances(), []);
        }
        gpu.finish_encoder(encoder);
    }
}

impl Into<Sprite> for RenderTarget {
    fn into(self) -> Sprite {
        return self.target;
    }
}
