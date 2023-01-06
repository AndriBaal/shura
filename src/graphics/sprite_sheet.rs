use crate::{Gpu, Sprite, Dimension};
use std::ops::{Index, IndexMut};

/// Collection of [Sprites](crate::Sprite) that will be loaded from the same image where all sprites have the same size.
pub struct SpriteSheet {
    sprites: Vec<Sprite>,
    sprite_size: Dimension<u32>
}

impl SpriteSheet {
    pub fn new(
        gpu: &Gpu,
        bytes: &[u8],
        sprites: Dimension<u32>,
        sprite_size: Dimension<u32>,
    ) -> SpriteSheet {
        let mut img = image::load_from_memory(bytes).unwrap();
        let amount = sprites.width * sprites.height;

        let mut sheet = SpriteSheet {
            sprites: Vec::with_capacity(amount as usize),
            sprite_size
        };

        for i in 0..sprites.height {
            for j in 0..sprites.width {
                let sprite = img.crop(
                    j * sprite_size.width,
                    i * sprite_size.height,
                    sprite_size.width,
                    sprite_size.height,
                );
                sheet
                    .sprites
                    .push(Sprite::from_image(gpu, sprite));
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

    // Getters
    pub fn sprite_size(&self) -> &Dimension<u32> {
        &self.sprite_size
    }

    pub fn sprite(&self, index: usize) -> &Sprite {
        &self.sprites[index]
    }

    pub fn sprites(&self) -> &[Sprite] {
        &self.sprites[..]
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
