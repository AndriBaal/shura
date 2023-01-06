use image::GenericImageView;
use crate::{
    Color, Dimension, Gpu, InstanceBuffer, Instances, Isometry, Matrix, Renderer, Uniform,
};
use std::num::NonZeroU32;

macro_rules! Where {
    (
    $a:lifetime >= $b:lifetime $(,)?
) => {
        &$b & $a()
    };
}

/// 2D Sprite used for rendering
pub struct Sprite {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    format: wgpu::TextureFormat,
    size: Dimension<u32>,
}

impl Sprite {
    /// Create a new [Sprite](crate::Sprite) from the raw image data.
    ///
    /// # Example
    /// ```
    /// let sprite = ctx.create_sprite(include_bytes!("path/to/my/image.png"));
    /// ```
    pub fn new(gpu: &Gpu, bytes: &[u8]) -> Self {
        let img = image::load_from_memory(bytes).unwrap();
        Self::from_image(gpu, img)
    }

    /// Compute a
    pub fn computed<'caller, F>(
        gpu: &Gpu,
        instances: &InstanceBuffer,
        fov: Dimension<f32>,
        camera: Isometry<f32>,
        texture_size: Dimension<u32>,
        clear_color: Option<Color>,
        compute: F,
    ) -> Self
    where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        let target = Sprite::empty(gpu, texture_size);
        target.draw(
            gpu,
            instances,
            fov,
            camera,
            texture_size,
            clear_color,
            compute,
        );
        return target;
    }

    pub fn empty(gpu: &Gpu, size: Dimension<u32>) -> Self {
        let (format, texture) = Self::create_texture(gpu, size);
        let bind_group = Self::create_group(gpu, &texture);
        Self {
            size,
            format,
            texture,
            bind_group,
        }
    }

    pub fn from_image(gpu: &Gpu, image: image::DynamicImage) -> Self {
        use wgpu::TextureFormat;
        let size = Dimension::new(image.width(), image.height());
        let (format, texture) = Self::create_texture(gpu, size);
        match gpu.config.format {
            TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb => {
                let image = image.to_bgra8();
                gpu.queue.write_texture(
                    texture.as_image_copy(),
                    &image,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: NonZeroU32::new(4 * image.width()),
                        rows_per_image: NonZeroU32::new(image.height()),
                    },
                    size.into(),
                );
            }
            _ => {
                let image = image.to_rgba8();
                gpu.queue.write_texture(
                    texture.as_image_copy(),
                    &image,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: NonZeroU32::new(4 * image.width()),
                        rows_per_image: NonZeroU32::new(image.height()),
                    },
                    size.into(),
                );
            }
        };

        let bind_group = Self::create_group(gpu, &texture);
        Self {
            bind_group,
            format,
            texture,
            size,
        }
    }

    pub fn draw<'caller, F>(
        &self,
        gpu: &Gpu,
        instances: &InstanceBuffer,
        fov: Dimension<f32>,
        camera: Isometry<f32>,
        texture_size: Dimension<u32>,
        clear_color: Option<Color>,
        compute: F,
    ) where
        F: for<'any> Fn(&mut Renderer<'any>, Instances, [Where!('caller >= 'any); 0]),
    {
        let mut encoder = gpu.encoder();
        let proj = Matrix::projection(fov);
        let view = Matrix::view(camera);
        let camera = Uniform::new_wgpu(
            &gpu.device,
            &gpu.queue,
            &gpu.defaults.vertex_uniform,
            view * proj,
        );
        let target_view = self.texture.create_view(&Default::default());

        let multisampled_frame_descriptor = &wgpu::TextureDescriptor {
            size: texture_size.into(),
            mip_level_count: 1,
            sample_count: gpu.defaults.sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        };

        let msaa = gpu
            .device
            .create_texture(multisampled_frame_descriptor)
            .create_view(&wgpu::TextureViewDescriptor::default());
        if let Some(color) = clear_color {
            Renderer::clear(&mut encoder, &target_view, &msaa, color);
        }
        {
            let mut renderer =
                Renderer::new_compute(&mut encoder, gpu, &target_view, &msaa, instances, &camera);
            compute(&mut renderer, 0..instances.instances(), []);
        }
        gpu.finish_enocder(encoder);
    }

    pub(crate) fn write_current_render(&mut self, encoder: &mut wgpu::CommandEncoder, gpu: &Gpu) {
        let defaults = &gpu.defaults;
        let target_view = self.texture.create_view(&Default::default());
        let relative_camera = &gpu.defaults.relative_camera;
        let mut renderer = Renderer::new_compute(
            encoder,
            gpu,
            &target_view,
            &gpu.defaults.target_msaa,
            &defaults.single_centered_instance,
            relative_camera.uniform(),
        );
        renderer.render_sprite(relative_camera.model(), &defaults.target);
        renderer.commit(&(0..1));
    }

    pub fn from_wgpu(
        size: Dimension<u32>,
        texture: wgpu::Texture,
        bind_group: wgpu::BindGroup,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            texture,
            format,
            size,
            bind_group,
        }
    }

    pub(crate) fn create_texture(
        gpu: &Gpu,
        size: Dimension<u32>,
    ) -> (wgpu::TextureFormat, wgpu::Texture) {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: size.into(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
        });

        return (gpu.config.format, texture);
    }

    pub(crate) fn create_group(gpu: &Gpu, texture: &wgpu::Texture) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &gpu.defaults.sprite_uniform,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.defaults.texture_sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        return bind_group;
    }

    /// Overwrite with an image of the same dimension
    pub fn write(&mut self, gpu: &Gpu, rgba: &image::RgbaImage) {
        gpu.queue.write_texture(
            self.texture.as_image_copy(),
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: NonZeroU32::new(4 * self.size.width),
                rows_per_image: NonZeroU32::new(self.size.height),
            },
            self.size.into(),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(target_os = "android"))]
    #[cfg(not(target_os = "ios"))]
    pub fn save(&self, gpu: &Gpu, file_name: &str) -> image::ImageResult<()> {
        self.to_image(gpu).save(file_name)
    }

    pub fn to_image(&self, gpu: &Gpu) -> image::DynamicImage {
        let o_texture_width = self.size.width;
        let texture_width = (o_texture_width as f64 / 64.0).ceil() as u32 * 64;
        let texture_height = self.size.height;
        let output_buffer_size = (4 * texture_width * texture_height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = gpu.device.create_buffer(&output_buffer_desc);

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("to_image_encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(4 * texture_width),
                    rows_per_image: NonZeroU32::new(texture_height),
                },
            },
            self.size.into(),
        );
        gpu.queue.submit(Some(encoder.finish()));

        let image = {
            let buffer_slice = output_buffer.slice(..);
            let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            gpu.device.poll(wgpu::Maintain::Wait);
            pollster::block_on(rx.receive()).unwrap().unwrap();
            let data = buffer_slice.get_mapped_range();
            let raw = data.as_ref().to_vec();
            match self.format {
                wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm => {
                    let image_buf =
                        image::ImageBuffer::from_vec(texture_width, texture_height, raw).unwrap();
                    let bgra = image::DynamicImage::ImageBgra8(image_buf);
                    let mut rgba = image::DynamicImage::ImageRgba8(bgra.to_rgba8());
                    rgba.crop(0, 0, o_texture_width, texture_height)
                }
                _ => {
                    let image_buf =
                        image::ImageBuffer::from_vec(texture_width, texture_height, raw).unwrap();
                    let mut rgba = image::DynamicImage::ImageRgba8(image_buf);
                    rgba.crop(0, 0, o_texture_width, texture_height)
                }
            }
        };
        output_buffer.unmap();
        return image;
    }
    // Getters

    #[inline]
    pub const fn size(&self) -> &Dimension<u32> {
        &self.size
    }

    #[inline]
    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    #[inline]
    pub const fn format(&self) -> &wgpu::TextureFormat {
        &self.format
    }

    #[inline]
    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}
