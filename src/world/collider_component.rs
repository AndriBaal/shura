use crate::{
    physics::{Collider, ColliderHandle, World},
    BaseComponent, InstanceData, Vector,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColliderComponent {
    pub collider_handle: ColliderHandle,
    pub tex: Vector<i32>,
    pub scale: Vector<f32>,
}

impl ColliderComponent {
    pub fn new(world: &mut World, collider: impl Into<Collider>) -> Self {
        world.create_collider_component(collider)
    }

    pub fn get<'a>(&self, world: &'a World) -> &'a Collider {
        world.collider(self.collider_handle).unwrap()
    }

    pub fn get_mut<'a>(&self, world: &'a mut World) -> &'a mut Collider {
        world.collider_mut(self.collider_handle).unwrap()
    }

    pub fn set_scale(&mut self, scale: Vector<f32>) {
        self.scale = scale;
    }

    pub const fn scale(&self) -> &Vector<f32> {
        &self.scale
    }

    pub fn set_tex(&mut self, tex: Vector<i32>) {
        self.tex = tex;
    }

    pub const fn tex(&self) -> &Vector<i32> {
        &self.tex
    }
}

impl BaseComponent for ColliderComponent {
    fn instance(&self, world: &World) -> InstanceData {
        if let Some(collider) = world.collider(self.collider_handle) {
            return InstanceData::new(
                *collider.position(),
                if collider.is_enabled() {
                    self.scale
                } else {
                    Vector::default()
                },
                self.tex,
            );
        }
        return InstanceData::default();
    }
}
