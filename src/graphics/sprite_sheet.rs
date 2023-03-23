use image::GenericImageView;
use crate::{Gpu, Sprite, Vector};
use std::ops::{Index, IndexMut};

/// Collection of [Sprites](crate::Sprite) that will be loaded from the same image where all sprites have the same size.
pub struct SpriteSheet {
    pub sprites: Vec<Sprite>,
    sprite_size: Vector<u32>,
}

impl SpriteSheet {
    pub fn new(
        gpu: &Gpu,
        bytes: &[u8],
        sprites: Vector<u32>,
        sprite_size: Vector<u32>,
    ) -> SpriteSheet {
        let mut img = image::load_from_memory(bytes).unwrap();
        let img_size = Vector::new(sprite_size.x * sprites.x, sprite_size.y * sprites.y);
        assert!(img_size.x <= img.width(), "SpriteSheet has invalid width!");
        assert!(img_size.y <= img.height(), "SpriteSheet has invalid height!");

        let amount = sprites.x * sprites.y;

        let mut sheet = SpriteSheet {
            sprites: Vec::with_capacity(amount as usize),
            sprite_size,
        };

        for i in 0..sprites.y {
            for j in 0..sprites.x {
                let sprite = img.crop(
                    j * sprite_size.x,
                    i * sprite_size.y,
                    sprite_size.x,
                    sprite_size.y,
                );
                sheet.sprites.push(Sprite::from_image(gpu, sprite));
            }
        }

        return sheet;
    }

    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    pub fn into_vec(self) -> Vec<Sprite> {
        self.sprites
    }

    pub fn sprite_size(&self) -> &Vector<u32> {
        &self.sprite_size
    }

    pub fn sprite(&self, index: usize) -> &Sprite {
        &self.sprites[index]
    }

    pub fn sprite_mut(&mut self, index: usize) -> &mut Sprite {
        &mut self.sprites[index]
    }
}

impl Index<usize> for SpriteSheet {
    type Output = Sprite;
    fn index<'a>(&'a self, i: usize) -> &'a Sprite {
        &self.sprites[i]
    }
}

impl IndexMut<usize> for SpriteSheet {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut Sprite {
        &mut self.sprites[i]
    }
}
