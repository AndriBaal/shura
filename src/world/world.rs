use crate::{physics::PhysicsComponent, BaseComponent, ComponentHandle};
use rapier2d::prelude::*;
use rustc_hash::FxHashMap;

type EventReceiver<T> = crossbeam::channel::Receiver<T>;
pub struct World {
    physics_pipeline: PhysicsPipeline,
    query_pipeline: QueryPipeline,
    gravity: Vector<Real>,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    event_collector: ChannelEventCollector,
    physics_priority: i16,
    pub event_receivers: (
        EventReceiver<CollisionEvent>,
        EventReceiver<ContactForceEvent>,
    ),

    component_mapping: FxHashMap<ColliderHandle, ComponentHandle>,
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum CollideType {
    Started,
    Stopped,
}

impl World {
    pub(crate) fn new() -> Self {
        let (collision_send, collision_recv) = crossbeam::channel::unbounded();
        let (contact_force_send, contact_force_recv) = crossbeam::channel::unbounded();
        let event_collector = ChannelEventCollector::new(collision_send, contact_force_send);
        Self {
            physics_pipeline: PhysicsPipeline::new(),
            query_pipeline: QueryPipeline::new(),
            gravity: vector![0.0, 0.0],
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            colliders: ColliderSet::new(),
            bodies: RigidBodySet::new(),
            event_collector,
            physics_priority: 1000,
            event_receivers: (collision_recv, contact_force_recv),
            component_mapping: Default::default(),
        }
    }

    pub(crate) fn create_body(&mut self, builder: impl Into<RigidBody>) -> RigidBodyHandle {
        self.bodies.insert(builder)
    }

    pub fn create_collider(
        &mut self,
        component: &PhysicsComponent,
        collider: &ColliderBuilder,
    ) -> ColliderHandle {
        if component.handle().id() == 0 {
            panic!("Initialize the component before adding additional colliders!");
        }

        let collider_handle = self.colliders.insert_with_parent(
            collider.build(),
            component.body_handle().unwrap(),
            &mut self.bodies,
        );

        self.component_mapping
            .insert(collider_handle, *component.handle());
        return collider_handle;
    }

    pub(crate) fn remove_body(&mut self, handle: RigidBodyHandle) {
        if let Some(body) = self.bodies.get(handle) {
            for collider in body.colliders() {
                self.component_mapping.remove(collider);
            }
        }

        self.bodies.remove(
            handle,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.component_mapping.remove(&handle);
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, true);
    }

    #[inline]
    pub fn create_joint(
        &mut self,
        rigid_body1: RigidBodyHandle,
        rigid_body2: RigidBodyHandle,
        joint: impl Into<GenericJoint>,
    ) -> ImpulseJointHandle {
        self.impulse_joints
            .insert(rigid_body1, rigid_body2, joint, true)
    }

    #[inline]
    pub fn remove_joint(&mut self, joint: ImpulseJointHandle) {
        self.impulse_joints.remove(joint, true);
    }

    #[inline]
    pub fn cast_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, f32)> {
        if let Some(collider) =
            self.query_pipeline
                .cast_ray(&self.bodies, &self.colliders, ray, max_toi, solid, filter)
        {
            return Some((self.component(&collider.0).unwrap(), collider.0, collider.1));
        }
        return None;
    }

    #[inline]
    pub fn cast_shape(
        &self,
        shape: &dyn Shape,
        position: &Isometry<f32>,
        velocity: &Vector<f32>,
        max_toi: f32,
        stop_at_penetration: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, TOI)> {
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
            return Some((self.component(&collider.0).unwrap(), collider.0, collider.1));
        }
        return None;
    }

    #[inline]
    pub fn cast_ray_and_get_normal(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle, RayIntersection)> {
        if let Some(collider) = self.query_pipeline.cast_ray_and_get_normal(
            &self.bodies,
            &self.colliders,
            ray,
            max_toi,
            solid,
            filter,
        ) {
            return Some((self.component(&collider.0).unwrap(), collider.0, collider.1));
        }
        return None;
    }

    #[inline]
    pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Vector<f32>) -> bool {
        if let Some(collider) = self.collider(collider_handle) {
            return collider
                .shape()
                .contains_point(collider.position(), &(point.into()));
        }
        return false;
    }

    #[inline]
    pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
        if let Some(collider) = self.collider(collider_handle) {
            return collider
                .shape()
                .intersects_ray(collider.position(), &ray, max_toi);
        }
        return false;
    }

    #[inline]
    pub fn test_filter(
        &mut self,
        filter: QueryFilter,
        handle: ColliderHandle,
        collider: &Collider,
    ) -> bool {
        filter.test(&self.bodies, handle, collider)
    }

    #[inline]
    pub fn intersections_with_ray(
        &self,
        ray: &Ray,
        max_toi: f32,
        solid: bool,
        filter: QueryFilter,
        mut callback: impl FnMut(ComponentHandle, ColliderHandle, RayIntersection) -> bool,
    ) {
        self.query_pipeline.intersections_with_ray(
            &self.bodies,
            &self.colliders,
            ray,
            max_toi,
            solid,
            filter,
            |collider, ray| callback(self.component(&collider).unwrap(), collider, ray),
        );
    }

    #[inline]
    pub fn intersections_with_shape(
        &self,
        shape_pos: &Isometry<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
        mut callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.query_pipeline.intersections_with_shape(
            &self.bodies,
            &self.colliders,
            shape_pos,
            shape,
            filter,
            |collider| callback(self.component(&collider).unwrap(), collider),
        );
    }

    #[inline]
    pub fn intersection_with_shape(
        &self,
        shape_pos: &Isometry<f32>,
        shape: &dyn Shape,
        filter: QueryFilter,
    ) -> Option<(ComponentHandle, ColliderHandle)> {
        if let Some(collider) = self.query_pipeline.intersection_with_shape(
            &self.bodies,
            &self.colliders,
            shape_pos,
            shape,
            filter,
        ) {
            let component = self.component(&collider).unwrap();
            return Some((component, collider));
        }
        return None;
    }

    #[inline]
    pub fn intersections_with_point(
        &self,
        point: &Point<f32>,
        filter: QueryFilter,
        mut callback: impl FnMut(ComponentHandle, ColliderHandle) -> bool,
    ) {
        self.query_pipeline.intersections_with_point(
            &self.bodies,
            &self.colliders,
            point,
            filter,
            |collider| callback(self.component(&collider).unwrap(), collider),
        );
    }

    pub(crate) fn step(&mut self, delta: f32) {
        self.integration_parameters.dt = delta;
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
            &(),
            &self.event_collector,
        );
        self.query_pipeline
            .update(&self.islands, &self.bodies, &self.colliders);
    }

    // Getters
    #[inline]
    pub fn collider<'a>(&'a self, collider_handle: ColliderHandle) -> Option<&'a Collider> {
        self.colliders.get(collider_handle)
    }

    #[inline]
    pub fn collider_mut<'a>(
        &'a mut self,
        collider_handle: ColliderHandle,
    ) -> Option<&'a mut Collider> {
        self.colliders.get_mut(collider_handle)
    }

    #[inline]
    pub(crate) fn rigid_body(&self, rigid_body_handle: RigidBodyHandle) -> &RigidBody {
        self.bodies.get(rigid_body_handle).unwrap()
    }

    #[inline]
    pub(crate) fn rigid_body_mut<'a>(
        &'a mut self,
        rigid_body_handle: RigidBodyHandle,
    ) -> Option<&'a mut RigidBody> {
        self.bodies.get_mut(rigid_body_handle)
    }

    #[inline]
    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.impulse_joints.get(joint)
    }

    #[inline]
    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.impulse_joints.get_mut(joint)
    }

    pub fn component(&self, collider_handle: &ColliderHandle) -> Option<ComponentHandle> {
        self.component_mapping.get(collider_handle).copied()
    }

    pub fn gravity(&self) -> &Vector<f32> {
        &self.gravity
    }

    pub fn physics_priority(&self) -> i16 {
        self.physics_priority
    }

    // Setters
    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.gravity = gravity;
    }

    pub fn set_physics_priority(&mut self, step: i16) {
        self.physics_priority = step;
    }
}
