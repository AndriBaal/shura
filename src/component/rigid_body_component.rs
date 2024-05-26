use crate::{
    component::{Component, MetaComponent},
    entity::EntityHandle,
    graphics::{Color, Instance2D, RenderGroup, SpriteArrayIndex, SpriteAtlas},
    math::{Vector2, AABB},
    physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle, World},
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RigidBodyComponentStatus {
    Initialized {
        rigid_body_handle: RigidBodyHandle,
    },
    Uninitialized {
        rigid_body: Box<RigidBody>,
        colliders: Vec<Collider>,
    },
}

impl RigidBodyComponentStatus {
    pub fn get<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return world.rigid_body(*rigid_body_handle).unwrap();
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => rigid_body,
        }
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return world.rigid_body_mut(*rigid_body_handle).unwrap();
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => rigid_body,
        }
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        match self {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                return world.attach_collider(*rigid_body_handle, collider)
            }
            RigidBodyComponentStatus::Uninitialized { colliders, .. } => {
                colliders.push(collider.into())
            }
        }
        None
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        match self {
            RigidBodyComponentStatus::Initialized { .. } => return world.detach_collider(collider),
            RigidBodyComponentStatus::Uninitialized { .. } => (),
        }
        None
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PhysicsComponentVisibility {
    Static(bool),
    Size(Vector2<f32>),
    Scaling,
    ColliderSize,
}

impl Default for PhysicsComponentVisibility {
    fn default() -> Self {
        Self::Static(true)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RigidBodyComponent {
    pub status: RigidBodyComponentStatus,
    pub scaling: Vector2<f32>,
    pub atlas: SpriteAtlas,
    pub color: Color,
    pub index: SpriteArrayIndex,
    pub visibility: PhysicsComponentVisibility,
}

impl RigidBodyComponent {
    pub fn new(
        rigid_body: impl Into<RigidBody>,
        colliders: impl IntoIterator<Item = impl Into<Collider>>,
    ) -> Self {
        Self {
            status: RigidBodyComponentStatus::Uninitialized {
                rigid_body: Box::new(rigid_body.into()),
                colliders: colliders.into_iter().map(|c| c.into()).collect(),
            },
            scaling: Vector2::new(1.0, 1.0),
            atlas: Default::default(),
            color: Color::WHITE,
            index: 0,
            visibility: PhysicsComponentVisibility::Static(true),
        }
    }

    pub fn handle(&self) -> Option<RigidBodyHandle> {
        match &self.status {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => Some(*rigid_body_handle),
            RigidBodyComponentStatus::Uninitialized { .. } => None,
        }
    }

    pub fn get<'a>(&'a self, world: &'a World) -> &'a RigidBody {
        self.status.get(world)
    }

    pub fn get_mut<'a>(&'a mut self, world: &'a mut World) -> &'a mut RigidBody {
        self.status.get_mut(world)
    }

    pub fn attach_collider(
        &mut self,
        world: &mut World,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        self.status.attach_collider(world, collider)
    }

    pub fn detach_collider(
        &mut self,
        world: &mut World,
        collider: ColliderHandle,
    ) -> Option<Collider> {
        self.status.detach_collider(world, collider)
    }

    pub fn with_scaling(mut self, scaling: Vector2<f32>) -> Self {
        self.scaling = scaling;
        self
    }

    pub fn set_scaling(&mut self, scaling: Vector2<f32>) {
        self.scaling = scaling;
    }

    pub const fn scaling(&self) -> &Vector2<f32> {
        &self.scaling
    }

    pub fn with_atlas(mut self, atlas: SpriteAtlas) -> Self {
        self.atlas = atlas;
        self
    }
    pub fn set_atlas(&mut self, atlas: SpriteAtlas) {
        self.atlas = atlas;
    }

    pub const fn atlas(&self) -> &SpriteAtlas {
        &self.atlas
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub const fn color(&self) -> &Color {
        &self.color
    }

    pub fn with_index(mut self, index: SpriteArrayIndex) -> Self {
        self.index = index;
        self
    }
    pub fn set_index(&mut self, index: SpriteArrayIndex) {
        self.index = index;
    }

    pub const fn index(&self) -> &SpriteArrayIndex {
        &self.index
    }

    pub fn set_visibility(&mut self, visibility: PhysicsComponentVisibility) {
        self.visibility = visibility;
    }

    pub fn instance(&self, world: &World) -> Instance2D {
        let rigid_body = match &self.status {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                world.rigid_body(*rigid_body_handle).unwrap()
            }
            RigidBodyComponentStatus::Uninitialized { rigid_body, .. } => &**rigid_body,
        };
        Instance2D::new(
            rigid_body.position().translation.vector,
            rigid_body.position().rotation.angle(),
            self.scaling,
            self.atlas,
            self.color,
            self.index,
        )
    }
}

impl MetaComponent for RigidBodyComponent {}
impl Component for RigidBodyComponent {
    type Instance = Instance2D;

    fn buffer(&self, world: &World, cam2d: &AABB, render_group: &mut RenderGroup<Self::Instance>)
    where
        Self: Sized,
    {
        match self.visibility {
            PhysicsComponentVisibility::Static(s) => {
                if s {
                    render_group.push(self.instance(world))
                }
            }
            PhysicsComponentVisibility::Size(size) => {
                let rigid_body = self.get(world);
                let aabb = AABB::from_center(*rigid_body.translation(), size);
                if aabb.intersects(cam2d) {
                    render_group.push(Instance2D::new(
                        rigid_body.position().translation.vector,
                        rigid_body.position().rotation.angle(),
                        self.scaling,
                        self.atlas,
                        self.color,
                        self.index,
                    ))
                }
            }
            PhysicsComponentVisibility::ColliderSize => {
                let mut aabb = AABB::default();
                let rigid_body = match &self.status {
                    RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                        let rigid_body = world.rigid_body(*rigid_body_handle).unwrap();
                        for collider_handle in rigid_body.colliders() {
                            let collider = world.collider(*collider_handle).unwrap();
                            aabb.combine(collider.compute_aabb().into());
                        }
                        rigid_body
                    }
                    RigidBodyComponentStatus::Uninitialized {
                        rigid_body,
                        colliders,
                    } => {
                        for collider in colliders {
                            aabb.combine(
                                collider
                                    .shape()
                                    .compute_aabb(&(rigid_body.position() * collider.position()))
                                    .into(),
                            )
                        }
                        &**rigid_body
                    }
                };

                if aabb.intersects(cam2d) {
                    render_group.push(Instance2D::new(
                        rigid_body.position().translation.vector,
                        rigid_body.position().rotation.angle(),
                        self.scaling,
                        self.atlas,
                        self.color,
                        self.index,
                    ))
                }
            }
            PhysicsComponentVisibility::Scaling => {
                let rigid_body = self.get(world);
                let aabb = AABB::from_center(*rigid_body.translation(), self.scaling);
                if aabb.intersects(cam2d) {
                    render_group.push(Instance2D::new(
                        rigid_body.position().translation.vector,
                        rigid_body.position().rotation.angle(),
                        self.scaling,
                        self.atlas,
                        self.color,
                        self.index,
                    ))
                }
            }
        }
    }

    fn init(&mut self, handle: EntityHandle, world: &mut World) {
        match self.status {
            RigidBodyComponentStatus::Initialized { .. } => {}
            RigidBodyComponentStatus::Uninitialized {
                ref rigid_body,
                ref colliders,
            } => {
                let rigid_body: &RigidBody = rigid_body;
                let rigid_body_handle =
                    world.add_rigid_body(rigid_body.clone(), colliders.clone(), &handle);
                self.status = RigidBodyComponentStatus::Initialized { rigid_body_handle };
            }
        }
    }

    fn finish(&mut self, world: &mut World) {
        match self.status {
            RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                if let Some((rigid_body, colliders)) = world.remove_rigid_body(rigid_body_handle) {
                    self.status = RigidBodyComponentStatus::Uninitialized {
                        rigid_body: Box::new(rigid_body),
                        colliders,
                    }
                }
            }
            RigidBodyComponentStatus::Uninitialized { .. } => (),
        }
    }

    fn remove_from_world(&self, world: &mut World) {
        world.remove_no_maintain_rigid_body(self)
    }
}
