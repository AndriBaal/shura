use crate::{Gpu, Sprite, Vector};
/// Collection of [Sprites](crate::Sprite) that will be loaded from the same image where all sprites have the same size.
pub struct SpriteSheet {
    sprite: Sprite,
    sprite_size: Vector<u32>,
    sprite_amount: Vector<u32>,
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

    pub fn offset(&self, index: Vector<u32>) -> Vector<f32> {
        todo!()
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
