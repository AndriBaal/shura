use std::ops::Deref;

use image::{DynamicImage, ImageBuffer};

use crate::{Color, Gpu, Sprite, Vector};
/// Collection of [Sprites](crate::Sprite) that will be loaded from the same image where all sprites have the same size.
pub struct SpriteSheet {
    pub sprite: Sprite,
    pub sprite_size: Vector<u32>,
    pub sprite_amount: Vector<u32>,
}

impl SpriteSheet {
    pub fn new(gpu: &Gpu, bytes: &[u8], sprite_size: Vector<u32>) -> SpriteSheet {
        let sprite = gpu.create_sprite(bytes);
        let sprite_amount = sprite.size().component_div(&sprite_size);

        return SpriteSheet {
            sprite,
            sprite_size,
            sprite_amount,
        };
    }

    pub fn from_amount(gpu: &Gpu, bytes: &[u8], sprite_amount: Vector<u32>) -> SpriteSheet {
        let sprite = gpu.create_sprite(bytes);
        let sprite_size = sprite.size().component_div(&sprite_amount);

        return SpriteSheet {
            sprite,
            sprite_size,
            sprite_amount,
        };
    }

    pub fn from_color(gpu: &Gpu, colors: &[Color]) -> Self {
        let img = ImageBuffer::from_fn(colors.len() as u32, 1, |x, _y| {
            colors[x as usize].into()
        });
        Self {
            sprite: Sprite::from_image(gpu, DynamicImage::ImageRgba8(img)),
            sprite_size: Vector::new(colors.len() as u32, 1),
            sprite_amount: Vector::new(colors.len() as u32, 1),
        }
    }

    pub fn tex_offset(&self, index: Vector<u32>) -> Vector<f32> {
        return Vector::new(1.0, 1.0)
            .component_div(&self.sprite_amount.cast::<f32>())
            .component_mul(&index.cast::<f32>());
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

    pub fn sprite(&self) -> &Sprite {
        &self.sprite
    }
}

impl Deref for SpriteSheet {
    type Target = Sprite;

    fn deref(&self) -> &Self::Target {
        &self.sprite
    }
}
