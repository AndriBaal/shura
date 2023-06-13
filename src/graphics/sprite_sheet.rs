use crate::{Gpu, Sprite, Vector};
use image::GenericImageView;
use std::{
    ops::{Index, IndexMut},
    slice::{Iter, IterMut},
};

/// Collection of [Sprites](crate::Sprite) that will be loaded from the same image where all sprites have the same size.
pub struct SpriteSheet {
    sprites: Vec<Sprite>,
    sprite_size: Vector<u32>,
    sprite_amount: Vector<usize>,
}

impl SpriteSheet {
    pub fn new(gpu: &Gpu, bytes: &[u8], sprite_size: Vector<u32>) -> SpriteSheet {
        let mut img = image::load_from_memory(bytes).unwrap();
        let img_size = Vector::new(img.width(), img.height());
        let sprite_amount = Vector::new(img_size.x / sprite_size.x, img_size.y / sprite_size.y);
        let amount = sprite_amount.x * sprite_amount.y;

        let mut sheet = SpriteSheet {
            sprites: Vec::with_capacity(amount as usize),
            sprite_size,
            sprite_amount: sprite_amount.cast::<usize>(),
        };

        for i in 0..sprite_amount.y {
            for j in 0..sprite_amount.x {
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

    pub fn from_amount(gpu: &Gpu, bytes: &[u8], sprite_amount: Vector<usize>) -> SpriteSheet {
        let mut img = image::load_from_memory(bytes).unwrap();
        let img_size = Vector::new(img.width(), img.height());
        let sprite_size = Vector::new(
            img_size.x / sprite_amount.x as u32,
            img_size.y / sprite_amount.y as u32,
        );

        let amount = sprite_amount.x * sprite_amount.y;

        let mut sheet = SpriteSheet {
            sprites: Vec::with_capacity(amount as usize),
            sprite_size,
            sprite_amount,
        };

        for i in 0..sprite_amount.y as u32 {
            for j in 0..sprite_amount.x as u32 {
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

    pub fn get(&self, pos: Vector<usize>) -> &Sprite {
        &self.sprites[pos.y * self.sprite_amount.x + pos.x]
    }

    pub fn get_mut(&mut self, pos: Vector<usize>) -> &mut Sprite {
        &mut self.sprites[pos.y * self.sprite_amount.x + pos.x]
    }

    pub fn len(&self) -> usize {
        self.sprites.len()
    }

    pub fn amount(&self) -> &Vector<usize> {
        &self.sprite_amount
    }

    pub fn to_vec(self) -> Vec<Sprite> {
        self.sprites
    }

    pub fn sprite_size(&self) -> &Vector<u32> {
        &self.sprite_size
    }

    pub fn first(&self) -> &Sprite {
        self.sprites.first().unwrap()
    }

    pub fn last(&self) -> &Sprite {
        self.sprites.last().unwrap()
    }

    pub fn first_mut(&mut self) -> &Sprite {
        self.sprites.first_mut().unwrap()
    }

    pub fn last_mut(&mut self) -> &Sprite {
        self.sprites.last_mut().unwrap()
    }

    pub fn sprite(&self, index: usize) -> &Sprite {
        &self.sprites[index]
    }

    pub fn sprite_mut(&mut self, index: usize) -> &mut Sprite {
        &mut self.sprites[index]
    }

    pub fn iter(&self) -> Iter<Sprite> {
        self.sprites.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<Sprite> {
        self.sprites.iter_mut()
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

impl Index<Vector<usize>> for SpriteSheet {
    type Output = Sprite;
    fn index<'a>(&'a self, i: Vector<usize>) -> &'a Sprite {
        self.get(i)
    }
}

impl IndexMut<Vector<usize>> for SpriteSheet {
    fn index_mut<'a>(&'a mut self, i: Vector<usize>) -> &'a mut Sprite {
        self.get_mut(i)
    }
}
