use rapier2d::{parry::query::ShapeCastOptions, pipeline::QueryFilter};

use crate::{
    ecs::Component,
    math::{Isometry2, Rotation2, Vector2},
    physics::{Physics, Shape},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Component)]
pub struct SimpleCharacterControllerComponent<S: Shape> {
    pub shape: S,
    pub position: Isometry2<f32>,
    pub linvel: Vector2<f32>,
}

impl<S: Shape> SimpleCharacterControllerComponent<S> {
    pub fn new(shape: S) -> Self {
        Self {
            shape,
            position: Isometry2::default(),
            linvel: Vector2::zeros(),
        }
    }

    pub fn set_shape(&mut self, shape: S) {
        self.shape = shape
    }

    pub fn shape(&self) -> &S {
        &self.shape
    }

    pub fn shape_mut(&mut self) -> &mut S {
        &mut self.shape
    }

    pub fn with_linvel(mut self, linvel: Vector2<f32>) -> Self {
        self.set_linvel(linvel);
        self
    }

    pub fn with_rotation(mut self, rotation: Rotation2<f32>) -> Self {
        self.set_rotation(rotation);
        self
    }

    pub fn with_translation(mut self, translation: Vector2<f32>) -> Self {
        self.set_translation(translation);
        self
    }

    pub fn with_position(mut self, position: Isometry2<f32>) -> Self {
        self.set_position(position);
        self
    }

    pub fn with_shape(mut self, shape: S) -> Self {
        self.shape = shape;
        self
    }

    pub fn set_rotation(&mut self, rotation: Rotation2<f32>) {
        self.position.rotation = rotation;
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.position = position
    }

    pub fn set_linvel(&mut self, linvel: Vector2<f32>) {
        self.linvel = linvel;
    }

    pub fn rotation(&self) -> Rotation2<f32> {
        self.position.rotation
    }

    pub fn translation(&self) -> Vector2<f32> {
        self.position.translation.vector
    }

    pub fn position(&self) -> &Isometry2<f32> {
        &self.position
    }

    pub const fn linvel(&self) -> &Vector2<f32> {
        &self.linvel
    }

    pub fn step(&mut self, time: f32, physics: &Physics, filter: QueryFilter) {
        // let mut result_translation = Vector2::zeros();
        let mut desired_translation = self.linvel * time;

        // if let Some(a) = physics.query_pipeline().intersection_with_shape(physics.rigid_bodies(), physics.colliders(), &translation_remaining.into(), &self.shape, filter) {
        //     println!("kjasdhgfkjhasgdfkjhasd");
        // }
        // self.set_translation(translation_remaining);

        let character_pos = &self.position;
        let character_shape = &self.shape;
        let bodies = physics.rigid_bodies();
        let colliders = physics.colliders();
        let queries = physics.query_pipeline();
        let translation_dir = desired_translation.normalize();

        if let Some((_handle, toi)) = queries.cast_shape(
            bodies,
            colliders,
            character_pos,
            &desired_translation.normalize(),
            character_shape,
            ShapeCastOptions::with_max_time_of_impact(desired_translation.norm()),
            filter,
        ) {
            let allowed_dist = toi.time_of_impact;
            desired_translation = translation_dir * allowed_dist;
            // TODO: Collision callback
        }

        self.set_translation(self.translation() + desired_translation);
    }
}
