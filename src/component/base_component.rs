use crate::{ComponentDerive, ComponentHandle, Isometry, Matrix, Rotation, Vector};
#[cfg(feature = "physics")]
use std::{
    cell::{Ref, RefMut},
    ops::{Deref, DerefMut},
};

#[cfg(feature = "physics")]
use crate::{
    physics::{Collider, ColliderBuilder, ColliderHandle, RcWorld, RigidBody, RigidBodyHandle},
    ComponentTypeId,
};

const NO_RIGID_BODY_PANIC: &'static str = "This body has no RigidBody or Collider!";

/// Easily create a [BaseComponent] with a position and render_scale.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone)]
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

    pub fn build(self) -> BaseComponent {
        self.into()
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
    handle: ComponentHandle,
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
    pub fn new_body(body: impl Into<RigidBody>, colliders: Vec<ColliderBuilder>) -> Self {
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
        self.handle = handle;
    }

    pub(crate) fn deinit(&mut self) {
        self.handle = ComponentHandle::INVALID;
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
                *world_wrapper.body(*body_handle).position(),
                Vector::new(1.0, 1.0),
            ),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => {
                Matrix::new(*body.position(), Vector::new(1.0, 1.0))
            }
        };
    }

    pub fn handle(&self) -> ComponentHandle {
        return self.handle;
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
                .body_mut(*body_handle)
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
                .body_mut(*body_handle)
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
                .body_mut(*body_handle)
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
            } => *world_wrapper.body(*body_handle).rotation(),
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
            } => *world_wrapper.body(*body_handle).translation(),
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
            } => *world_wrapper.body(*body_handle).position(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.position(),
        };
    }

    pub const fn render_scale(&self) -> &Vector<f32> {
        &self.render_scale
    }

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
    pub fn body(&self) -> impl Deref<Target = RigidBody> + '_ {
        self.try_body().expect(NO_RIGID_BODY_PANIC)
    }

    #[cfg(feature = "physics")]
    pub fn body_mut(&mut self) -> impl DerefMut<Target = RigidBody> + '_ {
        self.try_body_mut().expect(NO_RIGID_BODY_PANIC)
    }

    #[cfg(feature = "physics")]
    pub fn collider_handles(&self) -> impl Deref<Target = [ColliderHandle]> + '_ {
        self.try_collider_handles().expect(NO_RIGID_BODY_PANIC)
    }

    #[cfg(feature = "physics")]
    pub fn body_handle(&self) -> RigidBodyHandle {
        self.try_body_handle().expect(NO_RIGID_BODY_PANIC)
    }

    #[cfg(feature = "physics")]
    pub fn collider(&self, collider_handle: ColliderHandle) -> impl Deref<Target = Collider> + '_ {
        self.try_collider(collider_handle)
            .expect(NO_RIGID_BODY_PANIC)
    }

    #[cfg(feature = "physics")]
    pub fn collider_mut(
        &mut self,
        collider_handle: ColliderHandle,
    ) -> impl DerefMut<Target = Collider> + '_ {
        self.try_collider_mut(collider_handle)
            .expect(NO_RIGID_BODY_PANIC)
    }

    #[cfg(feature = "physics")]
    /// Returns the [RigidBody] if the component has one.
    pub fn try_body(&self) -> Option<impl Deref<Target = RigidBody> + '_> {
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
                return Some(BodyWrapper::Ref(world_wrapper.body(*body_handle)));
            }
            BodyStatus::RigidBodyPending { body, .. } => return Some(BodyWrapper::Owned(body)),
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    /// Returns the [RigidBody] if the component has one.
    pub fn try_body_mut(&mut self) -> Option<impl DerefMut<Target = RigidBody> + '_> {
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
                return Some(BodyWrapperMut::Ref(world_wrapper.body_mut(*body_handle)));
            }
            BodyStatus::RigidBodyPending { body, .. } => return Some(BodyWrapperMut::Owned(body)),
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    /// Returns a slice of [ColliderHandles](crate::physics::ColliderHandle) if the component has a [RigidBody](crate::physics::RigidBody) and
    /// the component is added to the [ComponentManager](crate::ComponentManager) of the [Scene](crate::Scene).
    pub fn try_collider_handles(&self) -> Option<impl Deref<Target = [ColliderHandle]> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                world_wrapper,
                body_handle,
                ..
            } => {
                return Some(Ref::map(world_wrapper.body(*body_handle), |body| {
                    body.colliders()
                }));
            }
            _ => {
                return None;
            }
        };
    }

    #[cfg(feature = "physics")]
    /// Returns the handle of the RigidBody if the component has a [RigidBody] and
    /// the component is added to the [ComponentManager](crate::ComponentManager) of the [Scene](crate::Scene).
    pub fn try_body_handle(&self) -> Option<RigidBodyHandle> {
        return match self.body {
            BodyStatus::RigidBody { body_handle, .. } => Some(body_handle),
            _ => None,
        };
    }

    #[cfg(feature = "physics")]
    /// Get a [Collider] that is attached to this components [RigidBody].
    pub fn try_collider(
        &self,
        collider_handle: ColliderHandle,
    ) -> Option<impl Deref<Target = Collider> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                body_handle,
                world_wrapper,
            } => {
                if let Some(collider) = world_wrapper.collider(collider_handle) {
                    if collider.parent().unwrap() == *body_handle {
                        return Some(collider);
                    }
                }
                return None;
            }
            _ => (),
        }
        return None;
    }

    #[cfg(feature = "physics")]
    /// Get a [Collider] that is attached to this components [RigidBody].
    pub fn try_collider_mut(
        &mut self,
        collider_handle: ColliderHandle,
    ) -> Option<impl DerefMut<Target = Collider> + '_> {
        match &mut self.body {
            BodyStatus::RigidBody {
                body_handle,
                world_wrapper,
            } => {
                if let Some(collider) = world_wrapper.collider_mut(collider_handle) {
                    if collider.parent().unwrap() == *body_handle {
                        return Some(collider);
                    }
                }
                return None;
            }
            _ => (),
        }
        return None;
    }

    #[cfg(feature = "physics")]
    pub(crate) fn remove_from_world(&mut self) {
        match &mut self.body {
            BodyStatus::RigidBody {
                ref world_wrapper,
                body_handle,
                ..
            } => self.body = world_wrapper.remove_body(*body_handle),
            _ => (),
        }
    }

    /// Check if this component has a [RigidBody].
    #[cfg(feature = "physics")]
    pub fn is_body(&self) -> bool {
        return match &self.body {
            BodyStatus::RigidBody { .. } => true,
            BodyStatus::RigidBodyPending { .. } => true,
            _ => false,
        };
    }

    #[cfg(feature = "physics")]
    pub(crate) fn add_to_world(&mut self, type_id: ComponentTypeId, world: RcWorld) {
        let temp = std::mem::replace(
            &mut self.body,
            BodyStatus::Position {
                position: Default::default(),
                matrix: Default::default(),
            },
        );
        match temp {
            BodyStatus::RigidBodyPending { body, colliders } => {
                let component_handle = self.handle();
                let body_handle = world.borrow_mut().create_body(*body);
                let mut world_mut = world.borrow_mut();
                for collider in colliders {
                    world_mut.create_collider(body_handle, component_handle, type_id, collider);
                }
                drop(world_mut);
                self.body = BodyStatus::RigidBody {
                    body_handle,
                    world_wrapper: WorldWrapper::Rc(world),
                };
            }
            _ => self.body = temp,
        }
    }

    #[cfg(all(feature = "physics", feature = "serde"))]
    /// Initialize the [RigidBody] after deserialization.
    pub fn init_body(&mut self, world: RcWorld) {
        match &mut self.body {
            BodyStatus::RigidBody { world_wrapper, .. } => *world_wrapper = WorldWrapper::Rc(world),
            _ => {}
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum BodyStatus {
    #[cfg(feature = "physics")]
    RigidBody {
        #[cfg_attr(feature = "serde", serde(default))]
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

impl ComponentDerive for BaseComponent {
    fn base(&self) -> &BaseComponent {
        self
    }

    fn base_mut(&mut self) -> &mut BaseComponent {
        self
    }
}

#[cfg(feature = "physics")]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum WorldWrapper {
    Rc(RcWorld),
    None,
}

impl Default for WorldWrapper {
    fn default() -> Self {
        Self::None
    }
}

impl WorldWrapper {
    const PANIC: &'static str =
        "Physic components can not be accessed before init_body was called on base!";
    pub fn collider(&self, handle: ColliderHandle) -> Option<Ref<Collider>> {
        match self {
            WorldWrapper::Rc(rc) => {
                return Ref::filter_map(rc.borrow(), |world| world.collider(handle)).ok();
            }
            _ => {
                panic!("{}", Self::PANIC)
            }
        }
    }

    pub fn collider_mut(&mut self, handle: ColliderHandle) -> Option<RefMut<Collider>> {
        match self {
            WorldWrapper::Rc(rc) => {
                return RefMut::filter_map(rc.borrow_mut(), |world| world.collider_mut(handle))
                    .ok();
            }
            _ => {
                panic!("{}", Self::PANIC)
            }
        }
    }

    pub fn body(&self, handle: RigidBodyHandle) -> Ref<RigidBody> {
        match self {
            WorldWrapper::Rc(rc) => {
                return Ref::map(rc.borrow(), |world| world.body(handle).unwrap());
            }
            _ => {
                panic!("{}", Self::PANIC)
            }
        }
    }

    pub fn body_mut(&mut self, handle: RigidBodyHandle) -> RefMut<RigidBody> {
        match self {
            WorldWrapper::Rc(rc) => {
                return RefMut::map(rc.borrow_mut(), |world| world.body_mut(handle).unwrap());
            }
            _ => {
                panic!("{}", Self::PANIC)
            }
        }
    }

    pub fn remove_body(&self, handle: RigidBodyHandle) -> BodyStatus {
        match self {
            WorldWrapper::Rc(rc) => {
                let (body, colliders) = rc.borrow_mut().remove_body(handle);
                return BodyStatus::RigidBodyPending {
                    body: Box::new(body),
                    colliders,
                };
            }
            _ => {
                panic!("{}", Self::PANIC)
            }
        }
    }
}
