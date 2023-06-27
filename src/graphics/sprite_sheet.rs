use image::DynamicImage;
use wgpu::{util::DeviceExt, ImageCopyTexture};

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
            bytes.push(vec![c.r, c.g, c.b, c.a]);
        }
        return Self::from_raw(gpu, &bytes, Vector::new(1, 1), sprite_size);
    }

    pub fn from_image(gpu: &Gpu, mut image: DynamicImage, sprite_size: Vector<u32>) -> Self {
        let size = Vector::new(image.width(), image.height());
        let sprite_amount = size.component_div(&sprite_size);
        let mut sprites: Vec<Vec<u8>> = vec![];
        for i in 0..sprite_amount.y as u32 {
            for j in 0..sprite_amount.x as u32 {
                let sprite = image.crop(
                    j * sprite_size.x,
                    i * sprite_size.y,
                    sprite_size.x,
                    sprite_size.y,
                );
                sprites.push(sprite.to_rgba8().to_vec());
            }
        }
        return Self::from_raw(gpu, &sprites, sprite_size, sprite_amount);
    }

    pub fn from_raw(
        gpu: &Gpu,
        // Every Sprite passed seperatley because split crop in image
        sprites: &[Vec<u8>],
        sprite_size: Vector<u32>,
        sprite_amount: Vector<u32>,
    ) -> Self {
        let amount = sprite_amount.x * sprite_amount.y;
        assert!(amount > 1, "SpriteSheet must atleast have to 2 sprites!");
        let size_hint_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("spritesheet_size_hint_buffer"),
                contents: bytemuck::cast_slice(&[sprite_amount.cast::<i32>(), Vector::new(0, 0)]), // Empty vec needed for 16 Byte alignment
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };
        let texture = gpu.device.create_texture(&texture_descriptor);

        for (layer, bytes) in sprites.iter().enumerate() {
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
                    bytes_per_row: Some(4 * sprite_size.x),
                    rows_per_image: Some(sprite_size.y),
                },
                wgpu::Extent3d {
                    width: sprite_size.x,
                    height: sprite_size.y,
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
            label: Some("sprite_sheet_bindgroup"),
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
