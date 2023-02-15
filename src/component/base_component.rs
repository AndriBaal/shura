use crate::{ComponentHandle, ComponentTypeId, Dimension, Isometry, Matrix, Rotation, Vector};
#[cfg(feature = "physics")]
use std::mem;

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

    pub fn matrix(&self, #[cfg(feature = "physics")] world: &World) -> Matrix {
        return match &self.body {
            BodyStatus::Position { matrix, .. } => *matrix,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body(*handle).unwrap();
                Matrix::new(*body.position())
            }
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

    pub fn set_rotation(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        rotation: Rotation<f32>,
    ) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.rotate(self.render_scale, rotation);
                position.rotation = rotation;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body_mut(*handle).unwrap();
                body.set_rotation(rotation, true)
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.set_rotation(rotation, true),
        }
    }

    pub fn set_translation(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        translation: Vector<f32>,
    ) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.translate(translation);
                position.translation.vector = translation;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body_mut(*handle).unwrap();
                body.set_translation(translation, true)
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.set_translation(translation, true),
        }
    }

    pub fn set_position(
        &mut self,
        #[cfg(feature = "physics")] world: &mut World,
        new_position: Isometry<f32>,
    ) {
        match &mut self.body {
            BodyStatus::Position { position, matrix } => {
                matrix.translate(position.translation.vector);
                matrix.rotate(self.render_scale, position.rotation);
                *position = new_position;
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body_mut(*handle).unwrap();
                body.set_position(new_position, true)
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.set_position(new_position, true),
        }
    }

    pub fn rotation<'a>(
        &'a self,
        #[cfg(feature = "physics")] world: &'a World,
    ) -> &'a Rotation<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => &position.rotation,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body(*handle).unwrap();
                body.rotation()
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.rotation(),
        };
    }

    pub fn translation<'a>(
        &'a self,
        #[cfg(feature = "physics")] world: &'a World,
    ) -> &'a Vector<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => &position.translation.vector,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body(*handle).unwrap();
                body.translation()
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.translation(),
        };
    }

    pub fn position<'a>(
        &'a self,
        #[cfg(feature = "physics")] world: &'a World,
    ) -> &'a Isometry<f32> {
        return match &self.body {
            BodyStatus::Position { position, .. } => &position,
            #[cfg(feature = "physics")]
            BodyStatus::RigidBody { handle } => {
                let body = world.rigid_body(*handle).unwrap();
                body.position()
            }
            #[cfg(feature = "physics")]
            BodyStatus::RigidBodyPending { body, .. } => body.position(),
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
    pub fn rigid_body<'a>(&'a self, world: &'a World) -> Option<&'a RigidBody> {
        return match &self.body {
            BodyStatus::RigidBody { handle } => world.rigid_body(*handle),
            BodyStatus::RigidBodyPending { body, .. } => Some(body),
            _ => None,
        };
    }

    #[cfg(feature = "physics")]
    pub fn rigid_body_mut<'a>(&'a mut self, world: &'a mut World) -> Option<&'a mut RigidBody> {
        return match &mut self.body {
            BodyStatus::RigidBody { handle } => world.rigid_body_mut(*handle),
            BodyStatus::RigidBodyPending { body, .. } => Some(body),
            _ => None,
        };
    }

    #[cfg(feature = "physics")]
    pub fn collider_handles<'a>(&'a self, world: &'a World) -> Option<&'a [ColliderHandle]> {
        self.rigid_body(world).and_then(|b| Some(b.colliders()))
    }

    #[cfg(feature = "physics")]
    pub fn rigid_body_handle(&self) -> Option<RigidBodyHandle> {
        return match self.body {
            BodyStatus::RigidBody { handle, .. } => Some(handle),
            _ => None,
        };
    }

    // pub fn off_screen(&self) -> bool {
    //     // TODO
    //     todo!();
    // }

    #[cfg(feature = "physics")]
    pub(crate) fn remove_from_world(&mut self, world: &mut World) {
        match std::mem::replace(
            &mut self.body,
            BodyStatus::RigidBody {
                handle: Default::default(),
            },
        ) {
            BodyStatus::RigidBody { handle } => {
                let (body, colliders) = world.remove_body(handle);
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
            BodyStatus::RigidBody {
                handle: Default::default(),
            },
        ) {
            BodyStatus::RigidBodyPending { body, colliders } => {
                self.body = BodyStatus::RigidBody {
                    handle: world.create_body(*body),
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
    RigidBody { handle: RigidBodyHandle },
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
