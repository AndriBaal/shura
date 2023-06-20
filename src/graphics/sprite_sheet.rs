use image::{DynamicImage, GenericImageView};
use wgpu::util::DeviceExt;

use crate::{Color, Gpu, Vector};
/// Collection of [Sprites](crate::Sprite) that will be loaded from the same image where all sprites have the same size.
pub struct SpriteSheet {
    _texture: wgpu::Texture,
    _size_hint_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    sprite_size: Vector<u32>,
    sprite_amount: Vector<u32>,
}

impl SpriteSheet {
    pub fn new(gpu: &Gpu, bytes: &[u8], sprite_size: Vector<u32>) -> Self {
        let img = image::load_from_memory(bytes).unwrap();
        Self::from_image(gpu, img, sprite_size)
    }

    pub fn from_amount(gpu: &Gpu, bytes: &[u8], sprite_amount: Vector<u32>) -> Self {
        let img = image::load_from_memory(bytes).unwrap();
        let sprite_size = Vector::new(
            img.width() / sprite_amount.x,
            img.height() / sprite_amount.y,
        );
        Self::from_image(gpu, img, sprite_size)
    }

    pub fn from_colors(gpu: &Gpu, colors: &[Color]) -> Self {
        let mut bytes = vec![];
        let sprite_size = Vector::new(colors.len() as u32, 1);
        for c in colors {
            bytes.extend_from_slice(&[c.r, c.g, c.b, c.a])
        }
        return Self::from_raw(gpu, &bytes, sprite_size, sprite_size);
    }

    pub fn from_image(gpu: &Gpu, image: DynamicImage, sprite_size: Vector<u32>) -> Self {
        let size = Vector::new(image.width(), image.height());
        let sprite_amount = size.component_div(&sprite_size);
        match gpu.config.format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                return Self::from_raw(gpu, &image.to_bgra8(), sprite_size, sprite_amount);
            }
            _ => {
                return Self::from_raw(gpu, &image.to_rgba8(), sprite_size, sprite_amount);
            }
        };
    }

    pub fn from_raw(
        gpu: &Gpu,
        bytes: &[u8],
        sprite_size: Vector<u32>,
        sprite_amount: Vector<u32>,
    ) -> Self {
        let amount = sprite_amount.x * sprite_amount.y;
        let size_hint_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("spritesheet_size_hint_buffer"),
                contents: bytemuck::cast_slice(&[sprite_amount]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("sprite_texture"),
            size: wgpu::Extent3d {
                width: sprite_size.x,
                height: sprite_size.y,
                depth_or_array_layers: amount,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.config.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            ..texture_descriptor
        });

        gpu.queue.write_texture(
            texture.as_image_copy(),
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * sprite_size.x),
                rows_per_image: Some(sprite_size.y),
            },
            wgpu::Extent3d {
                width: sprite_size.x,
                height: sprite_size.y,
                depth_or_array_layers: amount,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&gpu.base.texture_sampler),
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
            label: Some("sprite_shett_bindgroup"),
        });

        return Self {
            _texture: texture,
            _size_hint_buffer: size_hint_buffer,
            bind_group,
            sprite_size,
            sprite_amount,
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

// impl SpriteSheet {
//     pub fn new(gpu: &Gpu, bytes: &[u8], sprite_size: Vector<u32>) -> SpriteSheet {
//         let sprite = gpu.create_sprite(bytes);
//         let sprite_amount = sprite.size().component_div(&sprite_size);

//         return SpriteSheet {
//             sprite,
//             sprite_size,
//             sprite_amount,
//         };
//     }

//     pub fn from_amount(gpu: &Gpu, bytes: &[u8], sprite_amount: Vector<u32>) -> SpriteSheet {
//         let sprite = gpu.create_sprite(bytes);
//         let sprite_size = sprite.size().component_div(&sprite_amount);

//         return SpriteSheet {
//             sprite,
//             sprite_size,
//             sprite_amount,
//         };
//     }

//     pub fn from_color(gpu: &Gpu, colors: &[Color]) -> Self {
//         let img = ImageBuffer::from_fn(colors.len() as u32, 1, |x, _y| {
//             colors[x as usize].into()
//         });
//         Self {
//             sprite: Sprite::from_image(gpu, DynamicImage::ImageRgba8(img)),
//             sprite_size: Vector::new(colors.len() as u32, 1),
//             sprite_amount: Vector::new(colors.len() as u32, 1),
//         }
//     }

//     pub fn tex_offset(&self, index: Vector<u32>) -> Vector<f32> {
//         return Vector::new(1.0, 1.0)
//             .component_div(&self.sprite_amount.cast::<f32>())
//             .component_mul(&index.cast::<f32>());
//     }

//     pub fn sprite(&self) -> &Sprite {
//         &self.sprite
//     }
// }

// impl Deref for SpriteSheet {
//     type Target = Sprite;

//     fn deref(&self) -> &Self::Target {
//         &self.sprite
//     }
// }
