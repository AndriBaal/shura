use crate::{Camera, Gpu, GpuDefaults, RenderEncoder, Sprite, Vector};

/// Texture to render onto with a [RenderEncoder]
pub struct RenderTarget {
    target_msaa: wgpu::TextureView,
    target_view: wgpu::TextureView,
    target: Sprite,
}

impl RenderTarget {
    pub fn new(gpu: &Gpu, size: Vector<u32>) -> Self {
        // let size = Self::validate_webgl_size(size);
        let target = Sprite::empty(gpu, size);
        let target_view = target
            .texture()
            .create_view(&wgpu::TextureViewDescriptor::default());
        let target_msaa = Self::create_msaa(
            &gpu.device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            gpu.base.sample_count,
            size,
        );

        return Self {
            target_msaa,
            target,
            target_view,
        };
    }

    pub fn computed(
        gpu: &Gpu,
        defaults: &GpuDefaults,
        texture_size: Vector<u32>,
        compute: impl FnMut(&mut RenderEncoder),
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
        // let size = Self::validate_webgl_size(size);
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

    pub fn size(&self) -> Vector<u32> {
        self.target.size()
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.target_view
    }

    pub fn msaa(&self) -> &wgpu::TextureView {
        &self.target_msaa
    }

    pub fn draw<'caller>(
        &self,
        gpu: &Gpu,
        defaults: &GpuDefaults,
        compute: impl FnOnce(&mut RenderEncoder),
    ) {
        let mut encoder = RenderEncoder::new(gpu, defaults);
        compute(&mut encoder);
        encoder.finish();
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

impl Into<Sprite> for RenderTarget {
    fn into(self) -> Sprite {
        return self.target;
    }
}
