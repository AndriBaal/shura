use crate::{Gpu, RgbaColor, Vector};
use std::ops::Deref;

#[macro_export]
macro_rules! load_file {
    ($file:expr $(,)?) => {
        include_bytes!($file)
    };
}

#[macro_export]
macro_rules! load_file_root {
    ($file:expr $(,)?) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), '/', $file))
    };
}

#[macro_export]
macro_rules! sprite_file {
    ($file:expr) => {
        shura::SpriteBuilder::file(shura::load_file!($file))
    };
}

#[macro_export]
macro_rules! sprite_file_root {
    ($file:expr) => {
        shura::SpriteBuilder::file(shura::load_file_root!($file))
    };
}

pub struct SpriteBuilder<'a, D: Deref<Target = [u8]>> {
    pub size: Vector<u32>,
    pub sampler: wgpu::SamplerDescriptor<'a>,
    pub data: D,
    pub format: wgpu::TextureFormat,
}

impl<'a> SpriteBuilder<'a, image::RgbaImage> {
    pub fn file(bytes: &[u8]) -> Self {
        let image = image::load_from_memory(bytes).unwrap();
        let size = Vector::new(image.width(), image.height());
        return Self {
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data: image.to_rgba8(),
        };
    }

    pub fn image(image: image::DynamicImage) -> Self {
        let size = Vector::new(image.width(), image.height());
        return Self {
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data: image.to_rgba8(),
        };
    }
}

impl<'a> SpriteBuilder<'a, &'static [u8]> {
    pub fn empty(size: Vector<u32>) -> Self {
        return Self {
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data: &[],
        };
    }
}

impl<'a> SpriteBuilder<'a, Vec<u8>> {
    pub fn color(color: RgbaColor) -> Self {
        Self {
            size: Vector::new(1, 1),
            sampler: Sprite::DEFAULT_SAMPLER,
            data: vec![color.r, color.g, color.b, color.a],
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }
}

impl<'a> SpriteBuilder<'a, &'a [u8]> {
    pub fn raw(size: Vector<u32>, data: &'a [u8]) -> Self {
        return Self {
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data,
        };
    }
}

impl<'a, D: Deref<Target = [u8]>> SpriteBuilder<'a, D> {
    pub fn sampler(mut self, sampler: wgpu::SamplerDescriptor<'a>) -> Self {
        self.sampler = sampler;
        self
    }

    pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }
}

/// 2D Sprite used for rendering
#[derive(Debug)]
pub struct Sprite {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    _sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
    size: Vector<u32>,
}

impl Sprite {
    pub const DEFAULT_SAMPLER: wgpu::SamplerDescriptor<'static> = wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        // Copied from default ...
        lod_min_clamp: 0.0,
        lod_max_clamp: 32.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    };

    pub fn new<D: Deref<Target = [u8]>>(gpu: &Gpu, desc: SpriteBuilder<D>) -> Self {
        let texture = Self::create_texture(gpu, desc.format, desc.size);
        let (bind_group, sampler) = Self::create_bind_group(gpu, &texture, &desc.sampler);
        let sprite = Self {
            _sampler: sampler,
            size: desc.size,
            format: desc.format,
            texture,
            bind_group,
        };

        if desc.data.len() != 0 {
            sprite.write_raw(gpu, desc.size, &desc.data);
        }

        return sprite;
    }

    fn create_texture(gpu: &Gpu, format: wgpu::TextureFormat, size: Vector<u32>) -> wgpu::Texture {
        assert!(size.x != 0 && size.y != 0);
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite_texture"),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            format,
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

    fn create_bind_group(
        gpu: &Gpu,
        texture: &wgpu::Texture,
        sampler: &wgpu::SamplerDescriptor,
    ) -> (wgpu::BindGroup, wgpu::Sampler) {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = gpu.device.create_sampler(&sampler);
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &gpu.base.sprite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        return (bind_group, sampler);
    }

    /// Overwrite with an image of the same dimension
    pub fn write(&self, gpu: &Gpu, rgba: &image::RgbaImage) {
        Self::write_raw(&self, gpu, Vector::new(rgba.width(), rgba.height()), rgba)
    }

    pub fn write_raw(&self, gpu: &Gpu, size: Vector<u32>, data: &[u8]) {
        gpu.queue.write_texture(
            self.texture.as_image_copy(),
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.size.x),
                rows_per_image: Some(self.size.y),
            },
            wgpu::Extent3d {
                width: size.x,
                height: size.y,
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
            let mut raw = data.as_ref().to_vec();
            if self.format == wgpu::TextureFormat::Bgra8Unorm
                || self.format == wgpu::TextureFormat::Bgra8UnormSrgb
            {
                for chunk in raw.chunks_mut(4) {
                    let r = chunk[2];
                    let b = chunk[0];

                    chunk[0] = r;
                    chunk[2] = b;
                }
            }
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
        return self.format;
    }

    pub const fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}
