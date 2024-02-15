use nalgebra::{Translation2, UnitVector2};
use rapier2d::{geometry::ContactManifold, parry::query::DefaultQueryDispatcher, pipeline::QueryFilter};

use crate::{
    graphics::{Color, Instance2D, SpriteAtlas, SpriteSheetIndex},
    math::{Isometry2, Rotation2, Vector2},
    physics::{Shape, World},
};

use super::Component;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CharacterControllerComponent<S: Shape> {
    pub shape: S,
    scaling: Vector2<f32>,
    position: Isometry2<f32>,
    instance: Instance2D,
    active: bool,
    linvel: Vector2<f32>,
}

impl<S: Shape> CharacterControllerComponent<S> {
    pub fn new(shape: S) -> Self {
        Self {
            shape,
            scaling: Vector2::new(1.0, 1.0),
            position: Default::default(),
            instance: Instance2D::default(),
            active: true,
            linvel: Vector2::zeros(),
        }
    }

    pub fn set_shape(&mut self, shape: S) {
        self.shape = shape.into()
    }

    pub fn shape(&self) -> &S {
        &self.shape
    }

    pub fn shape_mut(&mut self) -> &mut S {
        &mut self.shape
    }

    pub fn with_scaling(mut self, scaling: Vector2<f32>) -> Self {
        self.set_scaling(scaling);
        self
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

    pub fn with_active(mut self, active: bool) -> Self {
        self.set_active(active);
        self
    }

    pub fn with_shape(mut self, shape: S) -> Self {
        self.shape = shape;
        self
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        self.instance
            .set_rotation_scaling(self.scaling, self.position.rotation);
    }

    pub fn set_rotation(&mut self, rotation: Rotation2<f32>) {
        self.position.rotation = rotation;
        self.instance.set_rotation_scaling(self.scaling, rotation);
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.instance.set_translation(translation);
        self.position.translation.vector = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.position = position;
        self.instance = Instance2D::new_position(position, self.scaling);
    }

    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.scaling = scaling;
        self.instance
            .set_rotation_scaling(self.scaling, self.position.rotation);
    }

    pub fn set_linvel(&mut self, linvel: Vector2<f32>) {
        self.linvel = linvel;
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn rotation(&self) -> Rotation2<f32> {
        self.position.rotation
    }

    pub fn translation(&self) -> Vector2<f32> {
        self.position.translation.vector
    }

    pub fn position(&self) -> Isometry2<f32> {
        self.position
    }

    pub const fn scaling(&self) -> &Vector2<f32> {
        &self.scaling
    }

    pub const fn linvel(&self) -> &Vector2<f32> {
        &self.linvel
    }

    pub fn instance(&self) -> Instance2D {
        self.instance
    }

    pub const fn color(&self) -> &Color {
        &self.instance.color
    }

    pub fn set_color(&mut self, color: Color) {
        self.instance.color = color;
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.set_color(color);
        self
    }

    pub const fn atlas(&self) -> &SpriteAtlas {
        &self.instance.atlas
    }

    pub fn set_atlas(&mut self, atlas: SpriteAtlas) {
        self.instance.atlas = atlas;
    }

    pub fn with_atlas(mut self, atlas: SpriteAtlas) -> Self {
        self.set_atlas(atlas);
        self
    }

    pub const fn index(&self) -> &SpriteSheetIndex {
        &self.instance.sprite_sheet_index
    }

    pub fn set_index(&mut self, index: SpriteSheetIndex) {
        self.instance.sprite_sheet_index = index;
    }

    pub fn with_index(mut self, index: SpriteSheetIndex) -> Self {
        self.set_index(index);
        self
    }

    pub fn step(&mut self, time: f32, world: &World, filter: QueryFilter) {
    //     let mut result_translation = Vector2::<f32>::zeros();
    //     // let dims = self.compute_dims(character_shape);

    //     // 1. Check and fix penetrations.
    //     // self.check_and_fix_penetrations();

    //     let mut translation_remaining = self.translation() + self.linvel * time;

    //     // let grounded_at_starting_pos = self.detect_grounded_status_and_apply_friction(
    //     //     dt,
    //     //     bodies,
    //     //     colliders,
    //     //     queries,
    //     //     character_shape,
    //     //     character_pos,
    //     //     &dims,
    //     //     filter,
    //     //     None,
    //     //     None,
    //     // );

    //     let mut max_iters = 20;
    //     let character_pos = &self.position;
    //     let character_shape = &self.shape;
    //     let offset = 0.0;
    //     // let mut kinematic_friction_translation = Vector::zeros();
    //     // let offset = self.offset.eval(dims.y);

    //     while let Some((translation_dir, translation_dist)) =
    //         UnitVector2::try_new_and_get(translation_remaining, 1.0e-5)
    //     {
    //         if max_iters == 0 {
    //             break;
    //         } else {
    //             max_iters -= 1;
    //         }

    //         // 2. Cast towards the movement direction.
    //         if let Some((handle, toi)) = world.query_pipeline().cast_shape(
    //             world.rigid_bodies(),
    //             world.colliders(),
    //             &(Translation2::from(result_translation) * character_pos),
    //             &translation_dir,
    //             character_shape,
    //             translation_dist + offset,
    //             false,
    //             filter,
    //         ) {
    //             // We hit something, compute the allowed self.
    //             let allowed_dist =
    //                 (toi.toi - (-toi.normal1.dot(&translation_dir)) * offset).max(0.0);
    //             let allowed_translation = *translation_dir * allowed_dist;
    //             result_translation += allowed_translation;
    //             translation_remaining -= allowed_translation;

    //             // events(CharacterCollision {
    //             //     handle,
    //             //     character_pos: Translation::from(result_translation) * character_pos,
    //             //     translation_applied: result_translation,
    //             //     translation_remaining,
    //             //     toi,
    //             // });

    //             // Try to go up stairs.
    //             if !self.handle_stairs(
    //                 bodies,
    //                 colliders,
    //                 queries,
    //                 character_shape,
    //                 &(Translation::from(result_translation) * character_pos),
    //                 &dims,
    //                 filter,
    //                 handle,
    //                 &mut translation_remaining,
    //                 &mut result,
    //             ) {
    //                 // No stairs, try to move along slopes.
    //                 translation_remaining =
    //                     self.handle_slopes(&toi, &translation_remaining, &mut result);
    //             }
    //         } else {
    //             // No interference along the path.
    //             result_translation += translation_remaining;
    //             translation_remaining.fill(0.0);
    //             break;
    //         }

    //         result.grounded = self.detect_grounded_status_and_apply_friction(
    //             dt,
    //             bodies,
    //             colliders,
    //             queries,
    //             character_shape,
    //             &(Translation::from(result_translation) * character_pos),
    //             &dims,
    //             filter,
    //             Some(&mut kinematic_friction_translation),
    //             Some(&mut translation_remaining),
    //         );

    //         if !self.slide {
    //             break;
    //         }
    //     }
    //     // If needed, and if we are not already grounded, snap to the ground.
    //     if grounded_at_starting_pos {
    //         self.snap_to_ground(
    //             bodies,
    //             colliders,
    //             queries,
    //             character_shape,
    //             &(Translation::from(result_translation) * character_pos),
    //             &dims,
    //             filter,
    //             &mut result,
    //         );
    //     }

    //     // Return the result.
    //     result




        let mut result_translation = Vector2::zeros();
        let mut translation_remaining = self.translation() + self.linvel * time;

        let mut max_iters = 20;

        while let Some((translation_dir, translation_dist)) =
            UnitVector2::try_new_and_get(translation_remaining, 1.0e-5)
        {
            if max_iters == 0 {
                break;
            } else {
                max_iters -= 1;
            }

            // 2. Cast towards the movement direction.
            if let Some((_handle, toi)) = world.query_pipeline().cast_shape(
                world.rigid_bodies(),
                world.colliders(),
                &(Translation2::from(result_translation) * &self.position),
                &translation_dir,
                &self.shape,
                translation_dist,
                false,
                filter,
            ) {
                // We hit something, compute the allowed self.
                let allowed_dist = (toi.toi - (-toi.normal1.dot(&translation_dir)) * 0.0).max(0.0);
                let allowed_translation = *translation_dir * allowed_dist;
                result_translation += allowed_translation;
                translation_remaining -= allowed_translation;

                // events(CharacterCollision {
                //     handle,
                //     character_pos: Translation::from(result_translation) * character_pos,
                //     translation_applied: result_translation,
                //     translation_remaining,
                //     toi,
                // });
            } else {
                // No interference along the path.
                result_translation += translation_remaining;
                translation_remaining.fill(0.0);
                break;
            }
        }

        // Return the result.
        self.set_translation(result_translation)
    }

}

impl<S: Shape> Component for CharacterControllerComponent<S> {
    type Instance = Instance2D;

    fn buffer(
        &self,
        _world: &World,
        render_group: &mut crate::graphics::RenderGroup<Self::Instance>,
    ) where
        Self: Sized,
    {
        render_group.push(self.instance);
    }

    fn init(&mut self, _handle: crate::entity::EntityHandle, _world: &mut World) {}
    fn finish(&mut self, _world: &mut World) {}
}
