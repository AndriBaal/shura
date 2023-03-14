use crate::{ComponentHandle, ComponentTypeId, Isometry, Matrix, Rotation, Vector};
#[cfg(feature = "physics")]
use std::{
    cell::{Ref, RefCell, RefMut},
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[cfg(feature = "physics")]
use crate::physics::{Collider, ColliderHandle, RigidBody, RigidBodyHandle, World};

/// Easily create a [BaseComponent] with a position and render_scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PositionBuilder {
    pub render_scale: Vector<f32>,
    pub position: Isometry<f32>,
}

impl Default for PositionBuilder {
    fn default() -> Self {
        Self {
            render_scale: Vector::new(1.0, 1.0),
            position: Default::default(),
        }
    }
}

impl PositionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render_scale(mut self, render_scale: Vector<f32>) -> Self {
        self.render_scale = render_scale;
        self
    }

    pub fn rotation(mut self, rotation: Rotation<f32>) -> Self {
        self.position.rotation = rotation;
        self
    }

    pub fn translation(mut self, translation: Vector<f32>) -> Self {
        self.position.translation.vector = translation;
        self
    }

    pub fn position(mut self, position: Isometry<f32>) -> Self {
        self.position = position;
        self
    }
}

impl Into<BaseComponent> for PositionBuilder {
    fn into(self) -> BaseComponent {
        return BaseComponent::new(self);
    }
}

/// Base of a component that is bound to a poisition on the screen, either by a
/// Position or a RigidBody.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseComponent {
    handle: Option<ComponentHandle>,
    render_scale: Vector<f32>,
    body: BodyStatus,
}

impl Default for BaseComponent {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

#[allow(unreachable_patterns)]
impl BaseComponent {
    pub fn new(pos: PositionBuilder) -> Self {
        let mut matrix = Matrix::default();
        matrix.translate(pos.position.translation.vector);
        matrix.rotate(pos.render_scale, pos.position.rotation);
        Self {
            handle: Default::default(),
            render_scale: pos.render_scale,
            body: BodyStatus::Position {
                matrix: matrix,
                position: pos.position,
            },
        }
    }

    #[cfg(feature = "physics")]
    pub fn new_rigid_body(body: impl Into<RigidBody>, colliders: Vec<impl Into<Collider>>) -> Self {
        Self {
            handle: Default::default(),
            render_scale: Vector::new(1.0, 1.0),
            body: BodyStatus::RigidBodyPending {
                body: Box::new(body.into()),
                colliders: colliders
                    .into_iter()
                    .map(|collider| collider.into())
                    .collect(),
            },
        }
    }

    pub(crate) fn init(&mut self, handle: ComponentHandle) {
        if self.handle.is_none() {
            self.handle = Some(handle);
        }
    }

    pub(crate) fn deinit(&mut self) {
        self.handle = None;
        #[cfg(feature = "physics")]
        match self.body {
            BodyStatus::RigidBody { .. } => self.remove_from_world(),
            _ => {}
        }
    }

    pub fn matrix(&self) -> Matrix {
        return match &self.body {
            BodyStatus::Position { matrix, .. } => *matrix,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => Matrix::new(
                *world_wrapper
                    .world()
                    .rigid_body(*body_handle)
                    .unwrap()
                    .position(),
            ),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => Matrix::new(*body.position()),
        };
    }

    pub fn handle(&self) -> Option<&ComponentHandle> {
        return self.handle.as_ref();
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.rotate(self.render_scale, rotation);
                position.rotation = rotation;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => world_wrapper
                .world_mut()
                .rigid_body_mut(*body_handle)
                .unwrap()
                .set_rotation(rotation, true),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.set_rotation(rotation, true),
        }
    }

    pub fn set_translation(&mut self, translation: Vector<f32>) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.translate(translation);
                position.translation.vector = translation;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => world_wrapper
                .world_mut()
                .rigid_body_mut(*body_handle)
                .unwrap()
                .set_translation(translation, true),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.set_translation(translation, true),
        }
    }

    pub fn set_position(&mut self, new_position: Isometry<f32>) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.translate(position.translation.vector);
                matrix.rotate(self.render_scale, position.rotation);
                *position = new_position;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => world_wrapper
                .world_mut()
                .rigid_body_mut(*body_handle)
                .unwrap()
                .set_position(new_position, true),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.set_position(new_position, true),
        }
    }

    pub fn rotation(&self) -> Rotation<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => position.rotation,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => *world_wrapper
                .world()
                .rigid_body(*body_handle)
                .unwrap()
                .rotation(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.rotation(),
        };
    }

    pub fn translation(&self) -> Vector<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => position.translation.vector,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => *world_wrapper
                .world()
                .rigid_body(*body_handle)
                .unwrap()
                .translation(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.translation(),
        };
    }

    pub fn position(&self) -> Isometry<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => *position,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => *world_wrapper
                .world()
                .rigid_body(*body_handle)
                .unwrap()
                .position(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.position(),
        };
    }

    pub const fn render_scale(&self) -> &Vector<f32> {
        &self.render_scale
    }

    // Setters

    pub fn set_scale(&mut self, render_scale: Vector<f32>) {
        self.render_scale = render_scale;
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.rotate(self.render_scale, position.rotation);
            }
            _ => {}
        }
    }

    #[cfg(feature = "physics")]
    /// Returns the [RigidBody] if the component has one.
    pub fn rigid_body(&self) -> Option<impl Deref<Target = RigidBody> + '_> {
        enum BodyWrapper<'a> {
            Owned(&'a Box<RigidBody>),
            Ref(Ref<'a, RigidBody>),
        }

        impl<'a> Deref for BodyWrapper<'a> {
            type Target = RigidBody;

            fn deref(&self) -> &Self::Target {
                return match self {
                    BodyWrapper::Owned(o) => o,
                    BodyWrapper::Ref(r) => r,
                };
            }
        }

        match &self.body {
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => {
                return Some(BodyWrapper::Ref(Ref::map(world_wrapper.world(), |world| {
                    world.rigid_body(*body_handle).unwrap()
                })));
            }
            BodyStatus::RigidBodyPending { body, .. } => return Some(BodyWrapper::Owned(body)),
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    /// Returns the [RigidBody] if the component has one.
    pub fn rigid_body_mut(&mut self) -> Option<impl DerefMut<Target = RigidBody> + '_> {
        enum BodyWrapperMut<'a> {
            Owned(&'a mut Box<RigidBody>),
            Ref(RefMut<'a, RigidBody>),
        }

        impl<'a> Deref for BodyWrapperMut<'a> {
            type Target = RigidBody;

            fn deref(&self) -> &Self::Target {
                return match self {
                    BodyWrapperMut::Owned(o) => o,
                    BodyWrapperMut::Ref(r) => r,
                };
            }
        }

        impl<'a> DerefMut for BodyWrapperMut<'a> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                return match self {
                    BodyWrapperMut::Owned(o) => o,
                    BodyWrapperMut::Ref(r) => r,
                };
            }
        }

        match &mut self.body {
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => {
                return Some(BodyWrapperMut::Ref(RefMut::map(
                    world_wrapper.world_mut(),
                    |world| world.rigid_body_mut(*body_handle).unwrap(),
                )));
            }
            BodyStatus::RigidBodyPending { body, .. } => return Some(BodyWrapperMut::Owned(body)),
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    /// Returns a slice of [ColliderHandles](crate::physics::ColliderHandle) if the component has a [RigidBody](crate::physics::RigidBody) and
    /// the component is added to the [ComponentManager](crate::ComponentManager) of the [Scene](crate::Scene).
    pub fn collider_handles(&self) -> Option<impl Deref<Target = [ColliderHandle]> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => {
                return Some(Ref::map(world_wrapper.world(), |world| {
                    world.rigid_body(*body_handle).unwrap().colliders()
                }));
            }
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    /// Returns the handle of the RigidBody if the component has a [RigidBody] and
    /// the component is added to the [ComponentManager](crate::ComponentManager) of the [Scene](crate::Scene).
    pub fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        return match self.body {
            BodyStatus::RigidBody { body_handle, .. } => Some(body_handle),
            _ => None,
        };
    }

    #[cfg(feature = "physics")]
    /// Get a [Collider] that is attached to this components [RigidBody].
    pub fn collider(
        &self,
        collider_handle: ColliderHandle,
    ) -> Option<impl Deref<Target = Collider> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                body_handle,
                world_wrapper,
            } => {
                return Ref::filter_map(world_wrapper.world(), |world| {
                    if let Some(collider) = world.collider(collider_handle) {
                        if collider.parent().unwrap() == *body_handle {
                            return Some(collider);
                        }
                    }
                    None
                })
                .ok();
            }
            _ => (),
        }
        return None;
    }

    #[cfg(feature = "physics")]
    /// Get a [Collider] that is attached to this components [RigidBody].
    pub fn collider_mut(
        &mut self,
        collider_handle: ColliderHandle,
    ) -> Option<impl DerefMut<Target = Collider> + '_> {
        match &mut self.body {
            BodyStatus::RigidBody {
                body_handle,
                world_wrapper,
            } => {
                return RefMut::filter_map(world_wrapper.world_mut(), |world| {
                    if let Some(collider) = world.collider_mut(collider_handle) {
                        if collider.parent().unwrap() == *body_handle {
                            return Some(collider);
                        }
                    }
                    None
                })
                .ok();
            }
            _ => (),
        }
        return None;
    }

    #[cfg(feature = "physics")]
    pub(crate) fn remove_from_world(&mut self) {
        let temp = std::mem::replace(
            &mut self.body,
            BodyStatus::Position {
                position: Default::default(),
                matrix: Default::default(),
            },
        );
        match temp {
            BodyStatus::RigidBody {
                mut world_wrapper,
                body_handle,
                ..
            } => {
                let (body, colliders) = world_wrapper.world_mut().remove_body(body_handle);
                self.body = BodyStatus::RigidBodyPending {
                    body: Box::new(body),
                    colliders,
                }
            }
            _ => self.body = temp,
        }
    }

    /// Check if this component has a [RigidBody].
    pub fn is_rigid_body(&self) -> bool {
        return match &self.body {
            BodyStatus::RigidBody { .. } => true,
            BodyStatus::RigidBodyPending { .. } => true,
            _ => false,
        };
    }

    #[cfg(feature = "physics")]
    pub(crate) fn add_to_world(&mut self, type_id: ComponentTypeId, world: Rc<RefCell<World>>) {
        let temp = std::mem::replace(
            &mut self.body,
            BodyStatus::Position {
                position: Default::default(),
                matrix: Default::default(),
            },
        );
        match temp {
            BodyStatus::RigidBodyPending { body, colliders } => {
                let component_handle = self.handle().copied().unwrap();
                let body_handle = world.borrow_mut().create_body(*body);
                let mut world_mut = world.borrow_mut();
                for collider in colliders {
                    world_mut.create_collider(body_handle, component_handle, type_id, collider);
                }
                drop(world_mut);
                self.body = BodyStatus::RigidBody {
                    body_handle,
                    world_wrapper: WorldWrapper::new(world),
                };
            }
            _ => self.body = temp,
        }
    }

    #[cfg(all(feature = "physics", feature = "serde"))]
    /// Initialize the [RigidBody] after deserialization.
    pub fn init_rigid_body(&mut self, world: Rc<RefCell<World>>) {
        match &mut self.body {
            BodyStatus::RigidBody { world_wrapper, .. } => {
                *world_wrapper = WorldWrapper::new(world)
            }
            _ => {}
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum BodyStatus {
    #[cfg(feature = "physics")]
    RigidBody {
        world_wrapper: WorldWrapper,
        body_handle: RigidBodyHandle,
    },
    #[cfg(feature = "physics")]
    RigidBodyPending {
        body: Box<RigidBody>,
        colliders: Vec<Collider>,
    },
    Position {
        position: Isometry<f32>,
        matrix: Matrix,
    },
}

#[cfg(feature = "physics")]
impl Drop for BaseComponent {
    fn drop(&mut self) {
        self.remove_from_world();
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct WorldWrapper {
    #[cfg(feature = "serde")]
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    world: Option<Rc<RefCell<World>>>,
    #[cfg(not(feature = "serde"))]
    world: Rc<RefCell<World>>,
}

impl WorldWrapper {
    pub fn new(rc: Rc<RefCell<World>>) -> Self {
        Self {
            #[cfg(feature = "serde")]
            world: Some(rc),
            #[cfg(not(feature = "serde"))]
            world: rc,
        }
    }
}

#[cfg(feature = "serde")]
impl WorldWrapper {
    pub fn world(&self) -> Ref<World> {
        return self
            .world
            .as_ref()
            .expect(
                "Physic components can not be accessed before init_rigid_body was called on base!",
            )
            .borrow();
    }

    pub fn world_mut(&mut self) -> RefMut<World> {
        return self
            .world
            .as_mut()
            .expect(
                "Physic components can not be accessed before init_rigid_body was called on base!",
            )
            .borrow_mut();
    }
}

#[cfg(not(feature = "serde"))]
impl WorldWrapper {
    pub fn world(&self) -> Ref<World> {
        return self.world.borrow();
    }

    pub fn world_mut(&mut self) -> RefMut<World> {
        return self.world.borrow_mut();
    }
}
