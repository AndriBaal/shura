use wgpu::util::DeviceExt;

use crate::{load_bytes, Gpu, RgbaColor, Vector2};
use std::{ops::Deref, path::Path};

pub struct SpriteBuilder<'a, D: Deref<Target = [u8]>> {
    pub label: Option<&'a str>,
    pub size: Vector2<u32>,
    pub sampler: wgpu::SamplerDescriptor<'a>,
    pub data: D,
    pub format: wgpu::TextureFormat,
}

impl<'a> SpriteBuilder<'a, image::RgbaImage> {
    pub async fn file(
        path: impl AsRef<Path>,
    ) -> SpriteBuilder<'a, image::ImageBuffer<image::Rgba<u8>, Vec<u8>>> {
        let bytes = load_bytes(path).await.unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        let size = Vector2::new(image.width(), image.height());
        return Self {
            label: None,
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data: image.to_rgba8(),
        };
    }

    pub fn bytes(bytes: &[u8]) -> Self {
        let image = image::load_from_memory(bytes).unwrap();
        let size = Vector2::new(image.width(), image.height());
        return Self {
            label: None,
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data: image.to_rgba8(),
        };
    }

    pub fn image(image: image::DynamicImage) -> Self {
        let size = Vector2::new(image.width(), image.height());
        return Self {
            label: None,
            size,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: Sprite::DEFAULT_SAMPLER,
            data: image.to_rgba8(),
        };
    }
}

impl<'a> SpriteBuilder<'a, &'static [u8]> {
    pub fn empty(size: Vector2<u32>) -> Self {
        return Self {
            label: None,
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
            label: None,
            size: Vector2::new(1, 1),
            sampler: Sprite::DEFAULT_SAMPLER,
            data: vec![color.r, color.g, color.b, color.a],
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        }
    }
}

impl<'a> SpriteBuilder<'a, &'a [u8]> {
    pub fn raw(size: Vector2<u32>, data: &'a [u8]) -> Self {
        return Self {
            label: None,
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

    pub fn label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }

    pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }
}

#[derive(Debug)]
pub struct Sprite {
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    format: wgpu::TextureFormat,
    size: Vector2<u32>,
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
        let texture = Self::create_texture(gpu, desc.label, desc.format, desc.size, &desc.data);
        let (view, bind_group, sampler) = Self::create_bind_group(gpu, &texture, &desc.sampler);
        return Self {
            _sampler: sampler,
            size: desc.size,
            format: desc.format,
            texture,
            view,
            bind_group,
        };
    }

    fn create_texture(
        gpu: &Gpu,
        label: Option<&str>,
        format: wgpu::TextureFormat,
        size: Vector2<u32>,
        data: &[u8],
    ) -> wgpu::Texture {
        assert!(size.x != 0 && size.y != 0);
        let texture = if data.is_empty() {
            gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: label,
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
            })
        } else {
            gpu.device.create_texture_with_data(
                &gpu.queue,
                &wgpu::TextureDescriptor {
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
                },
                data,
            )
        };

        return texture;
    }

    fn create_bind_group(
        gpu: &Gpu,
        texture: &wgpu::Texture,
        sampler: &wgpu::SamplerDescriptor,
    ) -> (wgpu::TextureView, wgpu::BindGroup, wgpu::Sampler) {
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

        return (view, bind_group, sampler);
    }

    pub fn write_image(&mut self, gpu: &Gpu, rgba: &image::RgbaImage) {
        Self::write(self, gpu, Vector2::new(rgba.width(), rgba.height()), rgba)
    }

    pub fn write(&mut self, gpu: &Gpu, size: Vector2<u32>, data: &[u8]) {
        gpu.queue.write_texture(
            self.texture.as_image_copy(),
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.format.block_size(None).unwrap() * size.x),
                rows_per_image: Some(size.y),
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
        let output_buffer_size = (self.format.block_size(None).unwrap()
            * texture_width
            * texture_height) as wgpu::BufferAddress;
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
                    bytes_per_row: Some(self.format.block_size(None).unwrap() * texture_width),
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

    pub const fn size(&self) -> Vector2<u32> {
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

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}
