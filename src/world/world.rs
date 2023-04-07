use crate::{BaseComponent, ComponentHandle, ComponentTypeId};
use rapier2d::prelude::*;
use rustc_hash::FxHashMap;

type EventReceiver<T> = crossbeam::channel::Receiver<T>;

struct WorldEvents {
    collision: EventReceiver<CollisionEvent>,
    _contact_force: EventReceiver<ContactForceEvent>,
    event_collector: ChannelEventCollector,
}

impl Default for WorldEvents {
    fn default() -> Self {
        let (collision_send, collision) = crossbeam::channel::unbounded();
        let (contact_force_send, _contact_force) = crossbeam::channel::unbounded();
        let event_collector = ChannelEventCollector::new(collision_send, contact_force_send);
        Self {
            collision,
            _contact_force,
            event_collector,
        }
    }
}

impl WorldEvents {
    fn collector(&self) -> &ChannelEventCollector {
        &self.event_collector
    }

    fn collision_event(&self) -> Result<CollisionEvent, crossbeam::channel::TryRecvError> {
        self.collision.try_recv()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct World {
    physics_priority: Option<i16>,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    component_mapping: FxHashMap<ColliderHandle, (ComponentTypeId, ComponentHandle)>,

    query_pipeline: QueryPipeline,
    gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    time_scale: f32,
    ccd_solver: CCDSolver,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    physics_pipeline: PhysicsPipeline,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default))]
    events: WorldEvents,
}

impl Clone for World {
    fn clone(&self) -> Self {
        Self {
            physics_priority: self.physics_priority.clone(),
            bodies: self.bodies.clone(),
            colliders: self.colliders.clone(),
            component_mapping: self.component_mapping.clone(),
            query_pipeline: self.query_pipeline.clone(),
            gravity: self.gravity.clone(),
            integration_parameters: self.integration_parameters.clone(),
            islands: self.islands.clone(),
            broad_phase: self.broad_phase.clone(),
            narrow_phase: self.narrow_phase.clone(),
            impulse_joints: self.impulse_joints.clone(),
            multibody_joints: self.multibody_joints.clone(),
            ccd_solver: self.ccd_solver.clone(),
            physics_pipeline: Default::default(),
            events: Default::default(),
            time_scale: self.time_scale,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum CollideType {
    Started,
    Stopped,
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
            events: Default::default(),
            component_mapping: Default::default(),
            physics_priority: Some(1000),
            gravity: vector![0.0, 0.0],
            time_scale: 1.0,
        }
    }

    pub(crate) fn create_body(&mut self, builder: RigidBody) -> RigidBodyHandle {
        self.bodies.insert(builder)
    }

    pub(crate) fn create_collider(
        &mut self,
        body_handle: RigidBodyHandle,
        component_handle: ComponentHandle,
        type_id: ComponentTypeId,
        collider: impl Into<Collider>,
    ) -> ColliderHandle {
        let collider_handle =
            self.colliders
                .insert_with_parent(collider, body_handle, &mut self.bodies);

        self.component_mapping
            .insert(collider_handle, (type_id, component_handle));
        return collider_handle;
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) -> Option<Collider> {
        self.component_mapping.remove(&handle);
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, true)
    }

    pub(crate) fn remove_body(&mut self, handle: RigidBodyHandle) -> (RigidBody, Vec<Collider>) {
        let colliders = self.bodies.get(handle).unwrap().colliders().to_vec();
        let collider = colliders
            .iter()
            .map(|collider_handle| {
                self.component_mapping.remove(collider_handle);
                self.colliders
                    .remove(*collider_handle, &mut self.islands, &mut self.bodies, false)
                    .unwrap()
            })
            .collect();

        let body = self
            .bodies
            .remove(
                handle,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            )
            .unwrap();
        return (body, collider);
    }

    pub fn create_joint(
        &mut self,
        component1: &BaseComponent,
        component2: &BaseComponent,
        joint: impl Into<GenericJoint>,
    ) -> ImpulseJointHandle {
        let body_handle1 = component1
            .rigid_body_handle()
            .expect("Cannot add a collider to a component with no RigidBody!");
        let body_handle2 = component2
            .rigid_body_handle()
            .expect("Cannot add a collider to a component with no RigidBody!");
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
    ) -> Option<(ComponentHandle, ColliderHandle, f32)> {
        if let Some(collider) =
            self.query_pipeline
                .cast_ray(&self.bodies, &self.colliders, ray, max_toi, solid, filter)
        {
            return Some((
                self.component_from_collider(&collider.0).unwrap().1,
                collider.0,
                collider.1,
            ));
        }
        return None;
    }

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
            return Some((
                self.component_from_collider(&collider.0).unwrap().1,
                collider.0,
                collider.1,
            ));
        }
        return None;
    }

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
            return Some((
                self.component_from_collider(&collider.0).unwrap().1,
                collider.0,
                collider.1,
            ));
        }
        return None;
    }

    pub fn intersects_point(&self, collider_handle: ColliderHandle, point: Vector<f32>) -> bool {
        if let Some(collider) = self.collider(collider_handle) {
            return collider
                .shape()
                .contains_point(collider.position(), &(point.into()));
        }
        return false;
    }

    pub fn intersects_ray(&self, collider_handle: ColliderHandle, ray: Ray, max_toi: f32) -> bool {
        if let Some(collider) = self.collider(collider_handle) {
            return collider
                .shape()
                .intersects_ray(collider.position(), &ray, max_toi);
        }
        return false;
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
        mut callback: impl FnMut(ComponentHandle, ColliderHandle, RayIntersection) -> bool,
    ) {
        self.query_pipeline.intersections_with_ray(
            &self.bodies,
            &self.colliders,
            ray,
            max_toi,
            solid,
            filter,
            |collider, ray| {
                callback(
                    self.component_from_collider(&collider).unwrap().1,
                    collider,
                    ray,
                )
            },
        );
    }

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
            |collider| callback(self.component_from_collider(&collider).unwrap().1, collider),
        );
    }

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
            let component = self.component_from_collider(&collider).unwrap().1;
            return Some((component, collider));
        }
        return None;
    }

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
            |collider| callback(self.component_from_collider(&collider).unwrap().1, collider),
        );
    }

    pub(crate) fn step(&mut self, delta: f32) {
        self.integration_parameters.dt = delta * self.time_scale;
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
            self.events.collector(),
        );
    }

    pub fn rigid_body(&self, rigid_body_handle: RigidBodyHandle) -> Option<&RigidBody> {
        return self.bodies.get(rigid_body_handle);
    }

    pub fn rigid_body_mut(&mut self, rigid_body_handle: RigidBodyHandle) -> Option<&mut RigidBody> {
        return self.bodies.get_mut(rigid_body_handle);
    }

    pub fn collider(&self, collider_handle: ColliderHandle) -> Option<&Collider> {
        self.colliders.get(collider_handle)
    }

    pub fn collider_mut(&mut self, collider_handle: ColliderHandle) -> Option<&mut Collider> {
        self.colliders.get_mut(collider_handle)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn bodies(&self) -> &RigidBodySet {
        &self.bodies
    }

    pub(crate) fn collision_event(
        &mut self,
    ) -> Result<CollisionEvent, crossbeam::channel::TryRecvError> {
        self.events.collision_event()
    }

    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.impulse_joints.get(joint)
    }

    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.impulse_joints.get_mut(joint)
    }

    pub fn component_from_collider(
        &self,
        collider_handle: &ColliderHandle,
    ) -> Option<(ComponentTypeId, ComponentHandle)> {
        self.component_mapping.get(collider_handle).cloned()
    }

    pub fn gravity(&self) -> Vector<f32> {
        self.gravity
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn physics_priority(&self) -> Option<i16> {
        self.physics_priority
    }

    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.gravity = gravity;
    }

    pub fn set_time_scale(&mut self, time_scale: f32) {
        self.time_scale = time_scale;
    }

    pub fn set_physics_priority(&mut self, step: Option<i16>) {
        self.physics_priority = step;
    }
}
