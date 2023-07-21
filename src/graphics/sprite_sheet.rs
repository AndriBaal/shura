use std::ops::Deref;
use wgpu::{util::DeviceExt, ImageCopyTexture};

use crate::{Gpu, RgbaColor, Vector};

pub type SpriteSheetIndex = Vector<u32>;

pub struct SpriteSheetBuilder<'a, D: Deref<Target = [u8]>> {
    pub sprite_size: Vector<u32>,
    pub sprite_amount: Vector<u32>,
    pub sampler: wgpu::SamplerDescriptor<'a>,
    pub data: Vec<D>,
}

impl<'a> SpriteSheetBuilder<'a, image::RgbaImage> {
    pub fn new(bytes: &[u8], sprite_size: Vector<u32>) -> Self {
        let img = image::load_from_memory(bytes).unwrap();
        Self::image(img, sprite_size)
    }

    pub fn image(mut image: image::DynamicImage, sprite_size: Vector<u32>) -> Self {
        let size = Vector::new(image.width(), image.height());
        let sprite_amount = size.component_div(&sprite_size);
        let mut data = vec![];
        for i in 0..sprite_amount.y as u32 {
            for j in 0..sprite_amount.x as u32 {
                let sprite = image.crop(
                    j * sprite_size.x,
                    i * sprite_size.y,
                    sprite_size.x,
                    sprite_size.y,
                );
                data.push(sprite.to_rgba8());
            }
        }
        Self {
            sprite_size,
            sprite_amount,
            sampler: Self::DEFAULT_SAMPLER,
            data,
        }
    }
}

impl<'a> SpriteSheetBuilder<'a, Vec<u8>> {
    pub fn colors(colors: &[RgbaColor]) -> Self {
        let mut data = vec![];
        for c in colors {
            data.push(vec![c.r, c.g, c.b, c.a]);
        }

        Self {
            sprite_size: Vector::new(1, 1),
            sprite_amount: Vector::new(colors.len() as u32, 1),
            sampler: Self::DEFAULT_SAMPLER,
            data,
        }
    }
}

impl<'a> SpriteSheetBuilder<'a, &'a [u8]> {
    pub fn raw(sprite_size: Vector<u32>, sprite_amount: Vector<u32>, data: Vec<&'a [u8]>) -> Self {
        return Self {
            sprite_size,
            sprite_amount,
            sampler: Self::DEFAULT_SAMPLER,
            data,
        };
    }
}

impl<'a, D: Deref<Target = [u8]>> SpriteSheetBuilder<'a, D> {
    pub const DEFAULT_SAMPLER: wgpu::SamplerDescriptor<'static> = wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        // Copied from default ..
        lod_min_clamp: 0.0,
        lod_max_clamp: 32.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    };

    pub fn sampler(mut self, sampler: wgpu::SamplerDescriptor<'a>) -> Self {
        self.sampler = sampler;
        self
    }
}

pub struct SpriteSheet {
    _texture: wgpu::Texture,
    _size_hint_buffer: wgpu::Buffer,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    sprite_size: Vector<u32>,
    sprite_amount: Vector<u32>,
}

impl SpriteSheet {
    pub fn new<D: Deref<Target = [u8]>>(gpu: &Gpu, desc: SpriteSheetBuilder<D>) -> Self {
        let amount = desc.sprite_amount.x * desc.sprite_amount.y;
        assert!(amount > 1, "SpriteSheet must atleast have to 2 sprites!");
        let size_hint_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("spritesheet_size_hint_buffer"),
                contents: bytemuck::cast_slice(&[desc.sprite_amount, Vector::new(0, 0)]), // Empty vec needed for 16 Byte alignment
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("sprite_texture"),
            size: wgpu::Extent3d {
                width: desc.sprite_size.x,
                height: desc.sprite_size.y,
                depth_or_array_layers: amount,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let texture = gpu.device.create_texture(&texture_descriptor);
        let sampler = gpu.device.create_sampler(&desc.sampler);

        for (layer, bytes) in desc.data.iter().enumerate() {
            gpu.queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * desc.sprite_size.x),
                    rows_per_image: Some(desc.sprite_size.y),
                },
                wgpu::Extent3d {
                    width: desc.sprite_size.x,
                    height: desc.sprite_size.y,
                    depth_or_array_layers: 1,
                },
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &size_hint_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
            layout: &gpu.base.sprite_sheet_layout,
            label: Some("sprite_sheet_bindgroup"),
        });

        return Self {
            _texture: texture,
            _size_hint_buffer: size_hint_buffer,
            _sampler: sampler,
            bind_group,
            sprite_size: desc.sprite_size,
            sprite_amount: desc.sprite_amount,
        };
    }

    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn len(&self) -> u32 {
        self.sprite_amount.x * self.sprite_amount.y
    }

    pub fn amount(&self) -> &Vector<u32> {
        &self.sprite_amount
    }

    pub fn sprite_size(&self) -> &Vector<u32> {
        &self.sprite_size
    }
}

// /// Create a [SpriteSheet] from multiple, by flattening all provided SpriteSheets to have their own row.
// pub fn from_multiple(gpu: &Gpu, sheets: &[&[u8]], sprite_size: Vector<u32>) -> Self {
//     let sprite_amount = size.component_div(&sprite_size);
//     let mut sprites: Vec<Vec<u8>> = vec![];

//     for i in 0..sprite_amount.y as u32 {
//         for j in 0..sprite_amount.x as u32 {
//             let sprite = image.crop(
//                 j * sprite_size.x,
//                 i * sprite_size.y,
//                 sprite_size.x,
//                 sprite_size.y,
//             );
//             sprites.push(sprite.as_rgba8().unwrap_or(&image.to_rgba8()).to_vec());
//         }
//     }
//     return Self::from_raw(gpu, &sprites, sprite_size, sprite_amount);
// }
