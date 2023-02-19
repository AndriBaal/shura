use crate::{ComponentHandle, ComponentTypeId, Dimension, Isometry, Matrix, Rotation, Vector};
use rapier2d::prelude::ColliderSet;
#[cfg(feature = "physics")]
use rapier2d::prelude::RigidBodySet;
#[cfg(feature = "physics")]
use std::{
    cell::{Ref, RefCell, RefMut},
    mem,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[cfg(feature = "physics")]
use crate::physics::{
    Collider, ColliderBuilder, ColliderHandle, RigidBody, RigidBodyHandle, World,
};

/// Easily create a [BaseComponent].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PositionBuilder {
    render_scale: Vector<f32>,
    position: Isometry<f32>,
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

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components X axis of its render_scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn render_scale_relative_width(mut self, window_size: Dimension<u32>) -> Self {
        self.render_scale.y = 1.0;
        self.render_scale.x = window_size.height as f32 / window_size.width as f32;
        self
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components Y axis of its render_scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn render_scale_relative_height(mut self, window_size: Dimension<u32>) -> Self {
        self.render_scale.x = 1.0;
        self.render_scale.y = window_size.width as f32 / window_size.height as f32;
        self
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

/// [BaseComponent] that only holds a position and a scale. This is very optimized for components with a
/// static position, since the matrix only updates when changing the component.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BaseComponent {
    handle: ComponentHandle,
    render_scale: Vector<f32>,
    body: BodyStatus,
    group_id: u32,
    type_id: ComponentTypeId,
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
            group_id: 0,
            type_id: Default::default(),
        }
    }

    #[cfg(feature = "physics")]
    pub fn new_rigid_body(body: impl Into<RigidBody>, colliders: Vec<ColliderBuilder>) -> Self {
        Self {
            handle: Default::default(),
            render_scale: Vector::new(1.0, 1.0),
            body: BodyStatus::RigidBodyPending {
                body: Box::new(body.into()),
                colliders: colliders.into_iter().map(|c| c.into()).collect(),
            },
            group_id: 0,
            type_id: Default::default(),
        }
    }

    pub(crate) fn init(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        handle: ComponentHandle,
        group_id: u32,
        type_id: ComponentTypeId,
    ) {
        if self.handle.id() == 0 {
            self.handle = handle;
            self.group_id = group_id;
            self.type_id = type_id;
            #[cfg(feature = "physics")]
            match &self.body {
                BodyStatus::RigidBodyPending { .. } => self.add_to_world(world),
                _ => {}
            }
        }
    }

    pub(crate) fn deinit(&mut self, #[cfg(feature = "physics")] world: &mut World) {
        self.handle = Default::default();
        self.group_id = 0;
        self.type_id = Default::default();
        #[cfg(feature = "physics")]
        match self.body {
            BodyStatus::RigidBody { .. } => self.remove_from_world(world),
            _ => {}
        }
    }

    pub fn matrix(&self) -> Matrix {
        return match &self.body {
            BodyStatus::Position { matrix, .. } => *matrix,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => Matrix::new(*body_set.borrow().get(*body_handle).unwrap().position()),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => Matrix::new(*body.position()),
        };
    }

    pub fn handle(&self) -> &ComponentHandle {
        if self.handle.id() == ComponentHandle::UNINITIALIZED_ID {
            panic!("Cannot get the handle from an unadded component!");
        }
        return &self.handle;
    }

    pub fn group_id(&self) -> u32 {
        return self.group_id;
    }

    pub fn type_id(&self) -> ComponentTypeId {
        return self.type_id;
    }

    pub fn set_rotation(&mut self, rotation: Rotation<f32>) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.rotate(self.render_scale, rotation);
                position.rotation = rotation;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => body_set
                .borrow_mut()
                .get_mut(*body_handle)
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
                body_set,
                body_handle,
                ..
            } => body_set
                .borrow_mut()
                .get_mut(*body_handle)
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
                body_set,
                body_handle,
                ..
            } => body_set
                .borrow_mut()
                .get_mut(*body_handle)
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
                body_set,
                body_handle,
                ..
            } => *body_set.borrow().get(*body_handle).unwrap().rotation(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.rotation(),
        };
    }

    pub fn translation(&self) -> Vector<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => position.translation.vector,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => *body_set.borrow().get(*body_handle).unwrap().translation(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.translation(),
        };
    }

    pub fn position(&self) -> Isometry<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => *position,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => *body_set.borrow().get(*body_handle).unwrap().position(),
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => *body.position(),
        };
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components X axis of its render_scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn scale_relative_width(&mut self, window_size: Dimension<u32>) {
        self.render_scale.y = 1.0;
        self.render_scale.x = window_size.height as f32 / window_size.width as f32;
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.rotate(self.render_scale, position.rotation);
            }
            _ => {}
        }
    }

    /// Used when the relative Camera is used, since the relative camera does not adjust its
    /// FOV, the bottom left is always (-0.5, -0.5). The components Y axis of its render_scale value
    /// automatically gets stretched so the aspect ratio of the rendered model remains the
    /// same.
    pub fn scale_relative_height(&mut self, window_size: Dimension<u32>) {
        self.render_scale.x = 1.0;
        self.render_scale.y = window_size.width as f32 / window_size.height as f32;
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.rotate(self.render_scale, position.rotation);
            }
            _ => (),
        }
    }

    #[inline]
    pub const fn render_scale(&self) -> &Vector<f32> {
        &self.render_scale
    }

    // Setters
    #[inline]
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
    pub fn rigid_body(&self) -> Option<impl Deref<Target = RigidBody> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => {
                return Some(BodyWrapper::Ref(Ref::map(body_set.borrow(), |body_set| {
                    body_set.get(*body_handle).unwrap()
                })));
            }
            BodyStatus::RigidBodyPending { body, .. } => return Some(BodyWrapper::Owned(body)),
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    pub fn rigid_body_mut(&mut self) -> Option<impl DerefMut<Target = RigidBody> + '_> {
        match &mut self.body {
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => {
                return Some(BodyWrapperMut::Ref(RefMut::map(
                    body_set.borrow_mut(),
                    |body_set| body_set.get_mut(*body_handle).unwrap(),
                )));
            }
            BodyStatus::RigidBodyPending { body, .. } => return Some(BodyWrapperMut::Owned(body)),
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    pub fn collider_handles(&self) -> Option<impl Deref<Target = [ColliderHandle]> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => {
                return Some(Ref::map(body_set.borrow(), |body_set| {
                    body_set.get(*body_handle).unwrap().colliders()
                }));
            }
            _ => return None,
        };
    }

    #[cfg(feature = "physics")]
    pub fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        return match self.body {
            BodyStatus::RigidBody { body_handle, .. } => Some(body_handle),
            _ => None,
        };
    }

    #[cfg(feature = "physics")]
    pub fn collider(
        &self,
        collider_handle: ColliderHandle,
    ) -> Option<impl Deref<Target = Collider> + '_> {
        match &self.body {
            BodyStatus::RigidBody {
                body_handle,
                collider_set,
                ..
            } => {
                return Ref::filter_map(collider_set.borrow(), |c| {
                    if let Some(collider) = c.get(collider_handle) {
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
    pub fn collider_mut(
        &mut self,
        collider_handle: ColliderHandle,
    ) -> Option<impl DerefMut<Target = Collider> + '_> {
        match &mut self.body {
            BodyStatus::RigidBody {
                body_handle,
                collider_set,
                ..
            } => {
                return RefMut::filter_map(collider_set.borrow_mut(), |c| {
                    if let Some(collider) = c.get_mut(collider_handle) {
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

    // #[cfg(feature = "physics")]
    // pub fn collider_mut(&mut self) -> impl Iterator<Item = &Collider> {
    //     // return std::iter::empty();
    // }

    #[cfg(feature = "physics")]
    pub(crate) fn remove_from_world(&mut self, world: &mut World) {
        match std::mem::replace(
            &mut self.body,
            BodyStatus::Position {
                position: Default::default(),
                matrix: Default::default(),
            },
        ) {
            BodyStatus::RigidBody {
                body_set,
                body_handle,
                ..
            } => {
                let (body, colliders) = world.remove_body(body_handle);
                self.body = BodyStatus::RigidBodyPending {
                    body: Box::new(body),
                    colliders,
                }
            }
            _ => {}
        };
    }

    #[cfg(feature = "physics")]
    pub(crate) fn add_to_world(&mut self, world: &mut World) {
        match mem::replace(
            &mut self.body,
            BodyStatus::Position {
                position: Default::default(),
                matrix: Default::default(),
            },
        ) {
            BodyStatus::RigidBodyPending { body, colliders } => {
                let body_handle = world.create_body(*body);
                let (body_set, collider_set) = world.clone_refrence();
                self.body = BodyStatus::RigidBody {
                    body_handle,
                    body_set,
                    collider_set,
                };
                for collider in colliders {
                    world.create_collider(self, collider);
                }
            }
            _ => {}
        };
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum BodyStatus {
    #[cfg(feature = "physics")]
    RigidBody {
        #[cfg_attr(feature = "serde", serde(skip))]
        collider_set: Rc<RefCell<ColliderSet>>,
        #[cfg_attr(feature = "serde", serde(skip))]
        body_set: Rc<RefCell<RigidBodySet>>,
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
enum BodyWrapper<'a> {
    Owned(&'a Box<RigidBody>),
    Ref(Ref<'a, RigidBody>),
}

#[cfg(feature = "physics")]
impl<'a> Deref for BodyWrapper<'a> {
    type Target = RigidBody;

    fn deref(&self) -> &Self::Target {
        return match self {
            BodyWrapper::Owned(o) => o,
            BodyWrapper::Ref(r) => r,
        };
    }
}

#[cfg(feature = "physics")]
enum BodyWrapperMut<'a> {
    Owned(&'a mut Box<RigidBody>),
    Ref(RefMut<'a, RigidBody>),
}

#[cfg(feature = "physics")]
impl<'a> Deref for BodyWrapperMut<'a> {
    type Target = RigidBody;

    fn deref(&self) -> &Self::Target {
        return match self {
            BodyWrapperMut::Owned(o) => o,
            BodyWrapperMut::Ref(r) => r,
        };
    }
}

#[cfg(feature = "physics")]
impl<'a> DerefMut for BodyWrapperMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return match self {
            BodyWrapperMut::Owned(o) => o,
            BodyWrapperMut::Ref(r) => r,
        };
    }
}
