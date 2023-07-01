use crate::{Gpu, RgbaColor, Vector};
use image::DynamicImage;

/// 2D Sprite used for rendering
#[derive(Debug)]
pub struct Sprite {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    size: Vector<u32>,
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

    pub(crate) fn empty(gpu: &Gpu, size: Vector<u32>) -> Self {
        assert!(size.x != 0 && size.y != 0);
        let texture = Self::create_texture(gpu, size);
        let bind_group = Self::create_bind_group(gpu, &texture);
        Self {
            size,
            texture,
            bind_group,
        }
    }

    pub fn from_image(gpu: &Gpu, image: DynamicImage) -> Self {
        let size = Vector::new(image.width(), image.height());
        return Self::from_raw(gpu, size, image.as_rgba8().unwrap_or(&image.to_rgba8()));
    }

    pub fn from_color(gpu: &Gpu, color: RgbaColor) -> Self {
        return Self::from_raw(
            gpu,
            Vector::new(1, 1),
            &[color.r, color.g, color.b, color.a],
        );
    }

    pub fn from_raw(gpu: &Gpu, size: Vector<u32>, data: &[u8]) -> Self {
        let texture = Self::create_texture(gpu, size);
        gpu.queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.x),
                rows_per_image: Some(size.y),
            },
            wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
        );

        let bind_group = Self::create_bind_group(gpu, &texture);
        Self {
            bind_group,
            texture,
            size,
        }
    }

    fn create_texture(gpu: &Gpu, size: Vector<u32>) -> wgpu::Texture {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_texture"),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        return texture;
    }

    fn create_bind_group(gpu: &Gpu, texture: &wgpu::Texture) -> wgpu::BindGroup {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &gpu.base.sprite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.base.texture_sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        return bind_group;
    }

    /// Overwrite with an image of the same dimension
    pub fn write(&self, gpu: &Gpu, rgba: &image::RgbaImage) {
        gpu.queue.write_texture(
            self.texture.as_image_copy(),
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.size.x),
                rows_per_image: Some(self.size.y),
            },
            wgpu::Extent3d {
                width: self.size.x,
                height: self.size.y,
                depth_or_array_layers: 1,
            },
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(not(target_os = "android"))]
    #[cfg(not(target_os = "ios"))]
    pub fn save(&self, gpu: &Gpu, file_name: &str) -> image::ImageResult<()> {
        self.to_image(gpu).to_rgba8().save(file_name)
    }

    pub fn to_image(&self, gpu: &Gpu) -> image::DynamicImage {
        let o_texture_width = self.size.x;
        let texture_width = (o_texture_width as f64 / 64.0).ceil() as u32 * 64;
        let texture_height = self.size.y;
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
                    bytes_per_row: Some(4 * texture_width),
                    rows_per_image: Some(texture_height),
                },
            },
            wgpu::Extent3d {
                width: self.size.x,
                height: self.size.y,
                depth_or_array_layers: 1,
            },
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
            let image_buf =
                image::ImageBuffer::from_vec(texture_width, texture_height, raw).unwrap();
            image::DynamicImage::ImageRgba8(image_buf).crop(0, 0, o_texture_width, texture_height)
        };

        output_buffer.unmap();
        return image;
    }

    pub const fn size(&self) -> Vector<u32> {
        self.size
    }

    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub const fn format(&self) -> wgpu::TextureFormat {
        return wgpu::TextureFormat::Rgba8UnormSrgb;
    }

    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}
