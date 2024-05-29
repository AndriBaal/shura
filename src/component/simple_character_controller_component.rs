use rapier2d::{parry::query::ShapeCastOptions, pipeline::QueryFilter};

use crate::{
    entity::EntityHandle,
    graphics::{Color, Instance2D, InstanceRenderGroup, SpriteAtlas, SpriteArrayIndex},
    math::{Isometry2, Vector2, AABB},
    physics::{Shape, World},
    component::{Component, MetaComponent, PhysicsComponentVisibility}
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
pub struct SimpleCharacterControllerComponent<S: Shape> {
    pub shape: S,
    instance: Instance2D,
    visibility: PhysicsComponentVisibility,
    linvel: Vector2<f32>,
}

impl<S: Shape> SimpleCharacterControllerComponent<S> {
    pub fn new(shape: S) -> Self {
        Self {
            shape,
            instance: Instance2D::default(),
            visibility: PhysicsComponentVisibility::default(),
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

    pub fn with_scaling(mut self, scaling: Vector2<f32>) -> Self {
        self.set_scaling(scaling);
        self
    }

    pub fn with_linvel(mut self, linvel: Vector2<f32>) -> Self {
        self.set_linvel(linvel);
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
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

    pub fn with_visibility(mut self, visibility: PhysicsComponentVisibility) -> Self {
        self.set_visibility(visibility);
        self
    }

    pub fn with_shape(mut self, shape: S) -> Self {
        self.shape = shape;
        self
    }

    pub fn set_visibility(&mut self, visibility: PhysicsComponentVisibility) {
        self.visibility = visibility;
    }

    pub fn set_rotation(&mut self, rotation: f32) {
        self.instance.rotation = rotation;
    }

    pub fn set_translation(&mut self, translation: Vector2<f32>) {
        self.instance.translation = translation;
    }

    pub fn set_position(&mut self, position: Isometry2<f32>) {
        self.instance.set_position(position);
    }

    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.instance.scaling = scaling;
    }

    pub fn set_linvel(&mut self, linvel: Vector2<f32>) {
        self.linvel = linvel;
    }

    pub fn visibility(&self) -> PhysicsComponentVisibility {
        self.visibility
    }

    pub fn rotation(&self) -> f32 {
        self.instance.rotation
    }

    pub fn translation(&self) -> Vector2<f32> {
        self.instance.translation
    }

    pub fn position(&self) -> Isometry2<f32> {
        self.instance.position()
    }

    pub const fn scaling(&self) -> &Vector2<f32> {
        &self.instance.scaling
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

    pub const fn index(&self) -> &SpriteArrayIndex {
        &self.instance.sprite_array_index
    }

    pub fn set_index(&mut self, index: SpriteArrayIndex) {
        self.instance.sprite_array_index = index;
    }

    pub fn with_index(mut self, index: SpriteArrayIndex) -> Self {
        self.set_index(index);
        self
    }

    pub fn step(&mut self, time: f32, world: &World, filter: QueryFilter) {
        // let mut result_translation = Vector2::zeros();
        let mut desired_translation = self.linvel * time;

        // if let Some(a) = world.query_pipeline().intersection_with_shape(world.rigid_bodies(), world.colliders(), &translation_remaining.into(), &self.shape, filter) {
        //     println!("kjasdhgfkjhasgdfkjhasd");
        // }
        // self.set_translation(translation_remaining);

        let character_pos = self.instance.position();
        let character_shape = &self.shape;
        let bodies = world.rigid_bodies();
        let colliders = world.colliders();
        let queries = world.query_pipeline();
        let translation_dir = desired_translation.normalize();

        if let Some((_handle, toi)) = queries.cast_shape(
            bodies,
            colliders,
            &character_pos,
            &desired_translation.normalize(),
            character_shape,
            ShapeCastOptions::with_max_time_of_impact(desired_translation.norm()),
            filter,
        ) {
            let allowed_dist = toi.time_of_impact;
            desired_translation = translation_dir * allowed_dist;
        }

        self.set_translation(self.translation() + desired_translation);
    }
}

impl<S: Shape> MetaComponent for SimpleCharacterControllerComponent<S> {}
impl<S: Shape> Component for SimpleCharacterControllerComponent<S> {
    type Instance = Instance2D;

    fn buffer(&self, _world: &World, cam2d: &AABB, render_group: &mut InstanceRenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        match self.visibility {
            PhysicsComponentVisibility::Static(b) => {
                if b {
                    render_group.push(self.instance);
                }
            }
            PhysicsComponentVisibility::Size(size) => {
                let aabb = AABB::from_center(self.translation(), size);
                if cam2d.intersects(&aabb) {
                    render_group.push(self.instance);
                }
            }
            PhysicsComponentVisibility::ColliderSize => {
                let aabb = self.shape.compute_aabb(&self.position()).into();
                if cam2d.intersects(&aabb) {
                    render_group.push(self.instance);
                }
            }
            PhysicsComponentVisibility::Scaling => {
                let aabb = AABB::from_center(self.translation(), *self.scaling());
                if cam2d.intersects(&aabb) {
                    render_group.push(self.instance);
                }
            }
        }
    }

    fn init(&mut self, _handle: EntityHandle, _world: &mut World) {}
    fn finish(&mut self, _world: &mut World) {}
}
