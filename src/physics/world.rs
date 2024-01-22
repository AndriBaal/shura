use crate::{
    entity::{EntityHandle, EntityIdentifier},
    math::{Isometry2, Point2, Vector2},
    physics::{RapierCollisionEvent, RapierContactForceEvent},
    time::TimeManager,
};
use rapier2d::{crossbeam, prelude::*};
use rustc_hash::FxHashMap;

type EventReceiver<T> = crossbeam::channel::Receiver<T>;
type ColliderMapping = FxHashMap<ColliderHandle, EntityHandle>;
type RigidBodyMapping = FxHashMap<RigidBodyHandle, EntityHandle>;

#[derive(Clone)]
pub struct CollectedEvents {
    collision: EventReceiver<RapierCollisionEvent>,
    contact_force: EventReceiver<RapierContactForceEvent>,
}

impl CollectedEvents {
    pub fn collisions(self, mut handle: impl FnMut(RapierCollisionEvent)) -> Self {
        while let Ok(collision_event) = self.collision.try_recv() {
            (handle)(collision_event);
        }
        self
    }

    pub fn contact_forces(self, mut handle: impl FnMut(RapierContactForceEvent)) -> Self {
        while let Ok(contact_force_event) = self.contact_force.try_recv() {
            (handle)(contact_force_event);
        }
        self
    }
}

pub trait WorldEvent: Sized {
    type Event;
    fn is<E1: EntityIdentifier, E2: EntityIdentifier>(&self, world: &World) -> Option<Self::Event>;
    fn has<E: EntityIdentifier>(&self, world: &World) -> Option<Self::Event>;
}

impl WorldEvent for RapierCollisionEvent {
    type Event = EntityCollisionEvent;

    fn is<E1: EntityIdentifier, E2: EntityIdentifier>(&self, world: &World) -> Option<Self::Event> {
        let collider1 = self.collider1();
        let collider2 = self.collider2();
        let entity1 = world.entity_from_collider(&collider1)?;
        let entity2 = world.entity_from_collider(&collider2)?;
        if entity1.entity_type_id() == E1::IDENTIFIER && entity2.entity_type_id() == E2::IDENTIFIER
        {
            return Some(EntityCollisionEvent {
                collider1,
                collider2,
                entity1: *entity1,
                entity2: *entity2,
                collision_type: if self.started() {
                    CollisionType::Started
                } else {
                    CollisionType::Stopped
                },
            });
        } else if entity2.entity_type_id() == E1::IDENTIFIER
            && entity1.entity_type_id() == E2::IDENTIFIER
        {
            return Some(EntityCollisionEvent {
                collider1: collider2,
                collider2: collider1,
                entity1: *entity2,
                entity2: *entity1,
                collision_type: if self.started() {
                    CollisionType::Started
                } else {
                    CollisionType::Stopped
                },
            });
        }

        None
    }

    fn has<E: EntityIdentifier>(&self, world: &World) -> Option<Self::Event> {
        let collider1 = self.collider1();
        let collider2 = self.collider2();
        let entity1 = world.entity_from_collider(&collider1)?;
        let entity2 = world.entity_from_collider(&collider2)?;
        if entity1.entity_type_id() == E::IDENTIFIER {
            return Some(EntityCollisionEvent {
                collider1,
                collider2,
                entity1: *entity1,
                entity2: *entity2,
                collision_type: if self.started() {
                    CollisionType::Started
                } else {
                    CollisionType::Stopped
                },
            });
        } else if entity2.entity_type_id() == E::IDENTIFIER {
            return Some(EntityCollisionEvent {
                collider1: collider2,
                collider2: collider1,
                entity1: *entity2,
                entity2: *entity1,
                collision_type: if self.started() {
                    CollisionType::Started
                } else {
                    CollisionType::Stopped
                },
            });
        }

        None
    }
}

impl WorldEvent for RapierContactForceEvent {
    type Event = EntityContactForceEvent;

    fn is<E1: EntityIdentifier, E2: EntityIdentifier>(&self, world: &World) -> Option<Self::Event> {
        let collider1 = self.collider1;
        let collider2 = self.collider2;
        let entity1 = world.entity_from_collider(&collider1)?;
        let entity2 = world.entity_from_collider(&collider2)?;
        if entity1.entity_type_id() == E1::IDENTIFIER && entity2.entity_type_id() == E2::IDENTIFIER
        {
            return Some(EntityContactForceEvent {
                collider1,
                collider2,
                entity1: *entity1,
                entity2: *entity2,
                total_force: self.total_force,
                total_force_magnitude: self.total_force_magnitude,
                max_force_direction: self.max_force_direction,
                max_force_magnitude: self.max_force_magnitude,
            });
        } else if entity2.entity_type_id() == E1::IDENTIFIER
            && entity1.entity_type_id() == E2::IDENTIFIER
        {
            return Some(EntityContactForceEvent {
                collider1: collider2,
                collider2: collider1,
                entity1: *entity2,
                entity2: *entity1,
                total_force: self.total_force,
                total_force_magnitude: self.total_force_magnitude,
                max_force_direction: self.max_force_direction,
                max_force_magnitude: self.max_force_magnitude,
            });
        }

        None
    }

    fn has<E: EntityIdentifier>(&self, world: &World) -> Option<Self::Event> {
        let collider1 = self.collider1;
        let collider2 = self.collider2;
        let entity1 = world.entity_from_collider(&collider1)?;
        let entity2 = world.entity_from_collider(&collider2)?;
        if entity1.entity_type_id() == E::IDENTIFIER {
            return Some(EntityContactForceEvent {
                collider1,
                collider2,
                entity1: *entity1,
                entity2: *entity2,
                total_force: self.total_force,
                total_force_magnitude: self.total_force_magnitude,
                max_force_direction: self.max_force_direction,
                max_force_magnitude: self.max_force_magnitude,
            });
        } else if entity2.entity_type_id() == E::IDENTIFIER {
            return Some(EntityContactForceEvent {
                collider1: collider2,
                collider2: collider1,
                entity1: *entity2,
                entity2: *entity1,
                total_force: self.total_force,
                total_force_magnitude: self.total_force_magnitude,
                max_force_direction: self.max_force_direction,
                max_force_magnitude: self.max_force_magnitude,
            });
        }

        None
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CollisionType {
    Started,
    Stopped,
}

struct EventCollector {
    collision: EventReceiver<RapierCollisionEvent>,
    contact_force: EventReceiver<RapierContactForceEvent>,
    collector: ChannelEventCollector,
}

impl Default for EventCollector {
    fn default() -> Self {
        let (collision_send, collision) = crossbeam::channel::unbounded();
        let (contact_force_send, contact_force) = crossbeam::channel::unbounded();
        let collector = ChannelEventCollector::new(collision_send, contact_force_send);
        Self {
            collision,
            contact_force,
            collector,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct EntityCollisionEvent {
    pub collider1: ColliderHandle,
    pub collider2: ColliderHandle,
    pub entity1: EntityHandle,
    pub entity2: EntityHandle,
    pub collision_type: CollisionType,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityContactForceEvent {
    pub collider1: ColliderHandle,
    pub collider2: ColliderHandle,
    pub entity1: EntityHandle,
    pub entity2: EntityHandle,
    pub total_force: Vector2<f32>,
    pub total_force_magnitude: f32,
    pub max_force_direction: Vector2<f32>,
    pub max_force_magnitude: f32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct World {
    pub time_scale: f32,
    pub gravity: Vector2<f32>,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    collider_mapping: ColliderMapping,
    rigid_body_mapping: RigidBodyMapping,

    query_pipeline: QueryPipeline,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    ccd_solver: CCDSolver,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    physics_pipeline: PhysicsPipeline,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    collector: EventCollector,
}

impl Clone for World {
    fn clone(&self) -> Self {
        Self {
            bodies: self.bodies.clone(),
            colliders: self.colliders.clone(),
            collider_mapping: self.collider_mapping.clone(),
            rigid_body_mapping: self.rigid_body_mapping.clone(),
            query_pipeline: self.query_pipeline.clone(),
            gravity: self.gravity,
            integration_parameters: self.integration_parameters,
            islands: self.islands.clone(),
            broad_phase: self.broad_phase.clone(),
            narrow_phase: self.narrow_phase.clone(),
            impulse_joints: self.impulse_joints.clone(),
            multibody_joints: self.multibody_joints.clone(),
            ccd_solver: self.ccd_solver.clone(),
            physics_pipeline: Default::default(),
            collector: Default::default(),
            time_scale: self.time_scale,
        }
    }
}

impl World {
    pub(crate) fn new() -> Self {
        Self {
            physics_pipeline: PhysicsPipeline::new(),
            query_pipeline: QueryPipeline::new(),
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            colliders: ColliderSet::new(),
            bodies: RigidBodySet::new(),
            collector: Default::default(),
            collider_mapping: Default::default(),
            rigid_body_mapping: Default::default(),
            gravity: Vector2::new(0.0, 0.0),
            time_scale: 1.0,
        }
    }

    pub(crate) fn add_collider(
        &mut self,
        entity_handle: EntityHandle,
        collider: Collider,
    ) -> ColliderHandle {
        let collider_handle = self.colliders.insert(collider.clone());
        self.collider_mapping.insert(collider_handle, entity_handle);
        collider_handle
    }

    pub(crate) fn add_rigid_body(
        &mut self,
        rigid_body: RigidBody,
        colliders: Vec<Collider>,
        entity_handle: EntityHandle,
    ) -> RigidBodyHandle {
        let rigid_body_handle = self.bodies.insert(rigid_body.clone());
        self.rigid_body_mapping
            .insert(rigid_body_handle, entity_handle);
        for collider in colliders {
            let collider_handle = self.colliders.insert_with_parent(
                collider.clone(),
                rigid_body_handle,
                &mut self.bodies,
            );
            self.collider_mapping.insert(collider_handle, entity_handle);
        }
        rigid_body_handle
    }

    pub(crate) fn remove_rigid_body(
        &mut self,
        handle: RigidBodyHandle,
    ) -> Option<(RigidBody, Vec<Collider>)> {
        self.rigid_body_mapping.remove(&handle);
        if let Some(rigid_body) = self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            false,
        ) {
            let mut colliders = vec![];
            for collider_handle in rigid_body.colliders() {
                if let Some(collider) = self.colliders.remove(
                    *collider_handle,
                    &mut self.islands,
                    &mut self.bodies,
                    true,
                ) {
                    colliders.push(collider);
                }
                self.collider_mapping.remove(collider_handle);
            }
            return Some((rigid_body, colliders));
        }
        None
    }

    pub(crate) fn remove_collider(&mut self, collider: ColliderHandle) -> Option<Collider> {
        self.collider_mapping.remove(&collider);
        if let Some(collider) =
            self.colliders
                .remove(collider, &mut self.islands, &mut self.bodies, false)
        {
            return Some(collider);
        }
        None
    }

    pub(crate) fn attach_collider(
        &mut self,
        rigid_body_handle: RigidBodyHandle,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        if let Some(entity) = self.rigid_body_mapping.get(&rigid_body_handle) {
            let collider =
                self.colliders
                    .insert_with_parent(collider, rigid_body_handle, &mut self.bodies);
            self.collider_mapping.insert(collider, *entity);
            return Some(collider);
        }
        None
    }

    pub(crate) fn detach_collider(&mut self, collider_handle: ColliderHandle) -> Option<Collider> {
        self.collider_mapping.remove(&collider_handle);

        self.colliders
            .remove(collider_handle, &mut self.islands, &mut self.bodies, true)
    }

    pub fn step(&mut self, time: &TimeManager) -> CollectedEvents {
        while let Ok(_event) = self.collector.collision.try_recv() {}
        while let Ok(_event) = self.collector.contact_force.try_recv() {}
        self.integration_parameters.dt = time.delta() * self.time_scale;
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &self.collector.collector,
        );
        self.events()
    }

    pub fn events(&self) -> CollectedEvents {
        CollectedEvents {
            collision: self.collector.collision.clone(),
            contact_force: self.collector.contact_force.clone(),
        }
    }

    #[cfg(all(feature = "serde", feature = "physics"))]
    pub(crate) fn remove_no_maintain(&mut self, component: &dyn crate::component::Component) {
        use crate::component::{
            ColliderComponent, ColliderComponentStatus, RigidBodyComponentStatus,
        };
        if let Some(component) = component.downcast_ref::<crate::component::RigidBodyComponent>() {
            match component.status {
                RigidBodyComponentStatus::Initialized { rigid_body_handle } => {
                    if let Some(rigid_body) = self.bodies.remove(
                        rigid_body_handle,
                        &mut self.islands,
                        &mut self.colliders,
                        &mut self.impulse_joints,
                        &mut self.multibody_joints,
                        true,
                    ) {
                        for collider_handle in rigid_body.colliders() {
                            self.collider_mapping.remove(collider_handle);
                        }
                    }
                }
                RigidBodyComponentStatus::Uninitialized { .. } => (),
            }
        } else if let Some(component) = component.downcast_ref::<ColliderComponent>() {
            match component.status {
                ColliderComponentStatus::Initialized { collider_handle } => {
                    self.collider_mapping.remove(&collider_handle);
                    self.colliders.remove(
                        collider_handle,
                        &mut self.islands,
                        &mut self.bodies,
                        false,
                    );
                }
                ColliderComponentStatus::Uninitialized { .. } => return,
            }
        }
    }

    // pub(crate) fn move_shape(
    //     &self,
    //     controller: &mut CharacterControllerEntity,
    //     time: &TimeManager,
    //     bodies: &RigidBodySet,
    //     colliders: &ColliderSet,
    //     queries: &QueryPipeline,
    //     character_shape: &dyn Shape,
    //     character_pos: &Isometry2<Real>,
    //     desired_translation: Vector2<Real>,
    //     filter: QueryFilter,
    //     mut events: impl FnMut(CharacterCollision, &World),
    // ) -> EffectiveCharacterMovement {

    // }

    pub fn entity_from_collider(&self, collider_handle: &ColliderHandle) -> Option<&EntityHandle> {
        self.collider_mapping.get(collider_handle)
    }

    pub fn create_joint(
        &mut self,
        body_handle1: RigidBodyHandle,
        body_handle2: RigidBodyHandle,
        joint: impl Into<GenericJoint>,
    ) -> ImpulseJointHandle {
        self.impulse_joints
            .insert(body_handle1, body_handle2, joint, true)
    }

    pub fn remove_joint(&mut self, joint: ImpulseJointHandle) -> Option<ImpulseJoint> {
        self.impulse_joints.remove(joint, true)
    }

    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(EntityHandle, ColliderHandle, f32)> {
        if let Some(collider) =
            self.query_pipeline
                .cast_ray(&self.bodies, &self.colliders, ray, max_toi, solid, filter)
        {
            if let Some(entity) = self.entity_from_collider(&collider.0) {
                return Some((*entity, collider.0, collider.1));
            }
        }
        None
    }

    pub fn cast_shape(
        &self,
        shape: &dyn Shape,
        position: &Isometry2<f32>,
        velocity: &Vector2<f32>,
        max_toi: f32,
        stop_at_penetration: bool,
        filter: QueryFilter,
    ) -> Option<(EntityHandle, ColliderHandle, TOI)> {
        if let Some(collider) = self.query_pipeline.cast_shape(
            &self.bodies,
            &self.colliders,
            position,
            velocity,
            shape,
            max_toi,
            stop_at_penetration,
            filter,
        ) {
            if let Some(entity) = self.entity_from_collider(&collider.0) {
                return Some((*entity, collider.0, collider.1));
            }
        }
        None
    }

    pub fn cast_ray_and_get_normal(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(EntityHandle, ColliderHandle, RayIntersection)> {
        if let Some(collider) = self.query_pipeline.cast_ray_and_get_normal(
            &self.bodies,
            &self.colliders,
            ray,
            max_toi,
            solid,
            filter,
        ) {
            if let Some(entity) = self.entity_from_collider(&collider.0) {
                return Some((*entity, collider.0, collider.1));
            }
        }
        None
    }

    pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Point2<f32>) -> bool {
        if let Some(collider) = self.collider(collider_handle) {
            return collider.shape().contains_point(collider.position(), &point);
        }
        false
    }

    pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
        if let Some(collider) = self.collider(collider_handle) {
            return collider
                .shape()
                .intersects_ray(collider.position(), &ray, max_toi);
        }
        false
    }

    pub fn test_filter(
        &self,
        filter: QueryFilter,
        handle: ColliderHandle,
        collider: &Collider,
    ) -> bool {
        filter.test(&self.bodies, handle, collider)
    }

    pub fn intersections_with_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
        mut callback: impl FnMut(EntityHandle, ColliderHandle, RayIntersection) -> bool,
    ) {
        self.query_pipeline.intersections_with_ray(
            &self.bodies,
            &self.colliders,
            ray,
            max_toi,
            solid,
            filter,
            |collider, ray| {
                if let Some(entity) = self.entity_from_collider(&collider) {
                    return callback(*entity, collider, ray);
                }
                true
            },
        );
    }

    pub fn intersections_with_shape(
        &self,
        shape_pos: &Isometry2<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
        mut callback: impl FnMut(EntityHandle, ColliderHandle) -> bool,
    ) {
        self.query_pipeline.intersections_with_shape(
            &self.bodies,
            &self.colliders,
            shape_pos,
            shape,
            filter,
            |collider| {
                if let Some(entity) = self.entity_from_collider(&collider) {
                    return callback(*entity, collider);
                }
                true
            },
        );
    }

    pub fn intersection_with_shape(
        &self,
        shape_pos: &Isometry2<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
    ) -> Option<(EntityHandle, ColliderHandle)> {
        if let Some(collider) = self.query_pipeline.intersection_with_shape(
            &self.bodies,
            &self.colliders,
            shape_pos,
            shape,
            filter,
        ) {
            if let Some(entity) = self.entity_from_collider(&collider) {
                return Some((*entity, collider));
            }
        }
        None
    }

    pub fn intersections_with_point(
        &self,
        point: &Point<f32>,
        filter: QueryFilter,
        mut callback: impl FnMut(EntityHandle, ColliderHandle) -> bool,
    ) {
        self.query_pipeline.intersections_with_point(
            &self.bodies,
            &self.colliders,
            point,
            filter,
            |collider| {
                if let Some(entity) = self.entity_from_collider(&collider) {
                    return callback(*entity, collider);
                }
                true
            },
        );
    }

    pub fn rigid_body(&self, body_handle: RigidBodyHandle) -> Option<&RigidBody> {
        return self.bodies.get(body_handle);
    }

    pub fn rigid_body_mut(&mut self, body_handle: RigidBodyHandle) -> Option<&mut RigidBody> {
        return self.bodies.get_mut(body_handle);
    }

    pub fn collider(&self, collider_handle: ColliderHandle) -> Option<&Collider> {
        self.colliders.get(collider_handle)
    }

    pub fn collider_mut(&mut self, collider_handle: ColliderHandle) -> Option<&mut Collider> {
        self.colliders.get_mut(collider_handle)
    }

    pub fn rigid_bodies(&self) -> &RigidBodySet {
        &self.bodies
    }

    pub fn colliders(&self) -> &ColliderSet {
        &self.colliders
    }

    pub fn integration_parameters_mut(&mut self) -> &mut IntegrationParameters {
        &mut self.integration_parameters
    }

    pub fn integration_parameters(&self) -> &IntegrationParameters {
        &self.integration_parameters
    }

    pub fn narrow_phase(&self) -> &NarrowPhase {
        &self.narrow_phase
    }

    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.impulse_joints.get(joint)
    }

    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.impulse_joints.get_mut(joint)
    }

    pub fn gravity(&self) -> Vector2<f32> {
        self.gravity
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn set_gravity(&mut self, gravity: Vector2<f32>) {
        self.gravity = gravity;
    }

    pub fn set_time_scale(&mut self, time_scale: f32) {
        self.time_scale = time_scale;
    }
}
