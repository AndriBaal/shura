#[cfg(not(target_arch = "wasm32"))]
use crate::assets::load_asset_bytes;
use crate::{
    assets::load_asset_bytes_async,
    graphics::{Gpu, RgbaColor},
    math::Vector2,
};
use std::ops::Deref;
use wgpu::ImageCopyTexture;

pub type SpriteSheetIndex = u32;
pub type SpriteSheetIndex2D = Vector2<u32>;

pub enum TileSize {
    Amount(Vector2<u32>),
    Size(Vector2<u32>),
}

pub struct SpriteSheetBuilder<'a, D: Deref<Target = [u8]>> {
    pub label: Option<&'a str>,
    pub sprite_size: Vector2<u32>,
    pub sprite_amount: Vector2<u32>,
    pub sampler: wgpu::SamplerDescriptor<'a>,
    pub data: Vec<D>,
    pub format: wgpu::TextureFormat,
}

impl<'a> SpriteSheetBuilder<'a, image::RgbaImage> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn asset(path: &str, size: TileSize) -> Self {
        let bytes = load_asset_bytes(path).unwrap();
        Self::bytes(&bytes, size)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn assets(paths: &[&str]) -> Self {
        let byte_array = paths
            .iter()
            .map(|path| load_asset_bytes(path).unwrap())
            .collect::<Vec<_>>();
        Self::byte_array(&byte_array)
    }

    pub async fn asset_async(path: &str, size: TileSize) -> Self {
        let bytes = load_asset_bytes_async(path).await.unwrap();
        Self::bytes(&bytes, size)
    }

    pub async fn assets_async(paths: &[&str]) -> Self {
        let mut byte_array = vec![];
        for path in paths {
            byte_array.push(load_asset_bytes_async(path).await.unwrap());
        }
        Self::byte_array(&byte_array)
    }

    pub fn bytes(bytes: &[u8], size: TileSize) -> Self {
        let img = image::load_from_memory(bytes).unwrap();
        Self::image(img, size)
    }

    pub fn byte_array(byte_array: &[Vec<u8>]) -> Self {
        let images: Vec<_> = byte_array
            .iter()
            .map(|bytes| image::load_from_memory(bytes).unwrap())
            .collect();
        Self::images(&images)
    }

    pub fn image(mut image: image::DynamicImage, size: TileSize) -> Self {
        let sheet_size = Vector2::new(image.width(), image.height());
        let (sprite_size, sprite_amount) = match size {
            TileSize::Amount(sprite_amount) => {
                (sheet_size.component_div(&sprite_amount), sprite_amount)
            }
            TileSize::Size(sprite_size) => (sprite_size, sheet_size.component_div(&sprite_size)),
        };
        let mut data = vec![];
        for i in 0..sprite_amount.y {
            for j in 0..sprite_amount.x {
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
            label: None,
            sprite_size,
            sprite_amount,
            sampler: Self::DEFAULT_SAMPLER,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            data,
        }
    }

    pub fn images(images: &[image::DynamicImage]) -> Self {
        assert!(images.len() > 1, "Images cannot be empty!");
        let sprite_size = Vector2::new(
            images.iter().max_by_key(|i| i.width()).unwrap().width(),
            images.iter().max_by_key(|i| i.height()).unwrap().height(),
        );
        let sprite_amount = Vector2::new(images.len() as u32, 1);
        let data = images.iter().map(|image| image.to_rgba8()).collect();
        Self {
            label: None,
            sprite_size,
            sprite_amount,
            sampler: Self::DEFAULT_SAMPLER,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            label: None,
            sprite_size: Vector2::new(1, 1),
            sprite_amount: Vector2::new(colors.len() as u32, 1),
            sampler: Self::DEFAULT_SAMPLER,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            data,
        }
    }
}

impl<'a> SpriteSheetBuilder<'a, &'a [u8]> {
    pub fn raw(
        sprite_size: Vector2<u32>,
        sprite_amount: Vector2<u32>,
        data: Vec<&'a [u8]>,
    ) -> Self {
        Self {
            label: None,
            sprite_size,
            sprite_amount,
            sampler: Self::DEFAULT_SAMPLER,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            data,
        }
    }

    pub fn empty(sprite_size: Vector2<u32>, sprite_amount: Vector2<u32>) -> Self {
        Self {
            label: None,
            sprite_size,
            sprite_amount,
            sampler: Self::DEFAULT_SAMPLER,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            data: vec![],
        }
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

    pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }

    pub fn label(mut self, label: Option<&'a str>) -> Self {
        self.label = label;
        self
    }
}

pub struct SpriteSheet {
    texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
    sprite_size: Vector2<u32>,
    sprite_amount: Vector2<u32>,
}

impl SpriteSheet {
    pub fn new<D: Deref<Target = [u8]>>(gpu: &Gpu, desc: SpriteSheetBuilder<D>) -> Self {
        let amount = desc.sprite_amount.x * desc.sprite_amount.y;
        let shared_assets = gpu.shared_assets();

        let texture_descriptor = wgpu::TextureDescriptor {
            label: desc.label,
            size: wgpu::Extent3d {
                width: desc.sprite_size.x,
                height: desc.sprite_size.y,
                // Fallback to ensure no crash because of only 2 sprite
                depth_or_array_layers: amount.max(2),
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: desc.format,
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
                    bytes_per_row: Some(
                        desc.format.block_copy_size(None).unwrap() * desc.sprite_size.x,
                    ),
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
            ],
            layout: &shared_assets.sprite_sheet_layout,
            label: Some("sprite_sheet_bind_group"),
        });

        Self {
            texture,
            _sampler: sampler,
            bind_group,
            sprite_size: desc.sprite_size,
            sprite_amount: desc.sprite_amount,
        }
    }

    pub fn write(
        &mut self,
        gpu: &Gpu,
        index: SpriteSheetIndex,
        size: Vector2<u32>,
        layers: u32,
        bytes: &[u8],
    ) {
        gpu.queue.write_texture(
            ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: index,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.texture.format().block_copy_size(None).unwrap() * size.x),
                rows_per_image: Some(size.y),
            },
            wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: layers,
            },
        );
    }

    pub const fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn len(&self) -> u32 {
        self.sprite_amount.x * self.sprite_amount.y
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len_2d(&self) -> &Vector2<u32> {
        &self.sprite_amount
    }

    pub fn index(&self, index_2d: SpriteSheetIndex2D) -> SpriteSheetIndex {
        index_2d.y * self.sprite_amount.x + index_2d.x
    }

    pub fn compute_index(sprite_amount_x: u32, index_2d: SpriteSheetIndex2D) -> SpriteSheetIndex {
        index_2d.y * sprite_amount_x + index_2d.x
    }

    pub fn sprite_size(&self) -> &Vector2<u32> {
        &self.sprite_size
    }
}

// pub fn from_multiple(gpu: &Gpu, sheets: &[&[u8]], sprite_size: Vector2<u32>) -> Self {
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
