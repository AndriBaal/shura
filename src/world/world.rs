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
    physics_priority: i16,
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
            physics_priority: 1000,
            events: Default::default(),
            component_mapping: Default::default(),
        }
    }

    pub(crate) fn create_body(&mut self, builder: impl Into<RigidBody>) -> RigidBodyHandle {
        self.bodies.insert(builder)
    }

    pub(crate) fn create_collider(
        &mut self,
        component: &BaseComponent,
        collider: impl Into<Collider>,
    ) -> ColliderHandle {
        let body_handle = component
            .rigid_body_handle()
            .expect("Cannot add a collider to a component with no RigidBody!");
        if component.handle().id() == 0 {
            panic!("Initialize the component before adding additional colliders!");
        }

        let collider_handle =
            self.colliders
                .insert_with_parent(collider, body_handle, &mut self.bodies);

        self.component_mapping
            .insert(collider_handle, (component.type_id(), *component.handle()));
        return collider_handle;
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

    pub fn remove_collider(&mut self, handle: ColliderHandle) -> Option<Collider> {
        self.component_mapping.remove(&handle);
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, true)
    }

    #[inline]
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

    #[inline]
    pub fn remove_joint(&mut self, joint: ImpulseJointHandle) -> Option<ImpulseJoint> {
        self.impulse_joints.remove(joint, true)
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
            return Some((self.component(&collider.0).unwrap().1, collider.0, collider.1));
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
            return Some((self.component(&collider.0).unwrap().1, collider.0, collider.1));
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
            return Some((self.component(&collider.0).unwrap().1, collider.0, collider.1));
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
        &self,
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
            |collider, ray| callback(self.component(&collider).unwrap().1, collider, ray),
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
            |collider| callback(self.component(&collider).unwrap().1, collider),
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
            let component = self.component(&collider).unwrap().1;
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
            |collider| callback(self.component(&collider).unwrap().1, collider),
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
            Some(&mut self.query_pipeline),
            &(),
            self.events.collector(),
        );
    }

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
    pub(crate) fn rigid_body(&self, body_handle: RigidBodyHandle) -> Option<&RigidBody> {
        self.bodies.get(body_handle)
    }

    #[inline]
    pub(crate) fn rigid_body_mut<'a>(
        &'a mut self,
        body_handle: RigidBodyHandle,
    ) -> Option<&'a mut RigidBody> {
        self.bodies.get_mut(body_handle)
    }

    #[inline]
    pub(crate) fn bodies(&self) -> impl Iterator<Item = (RigidBodyHandle, &RigidBody)> {
        self.bodies.iter()
    }

    #[inline]
    pub(crate) fn collision_event(
        &mut self,
    ) -> Result<CollisionEvent, crossbeam::channel::TryRecvError> {
        self.events.collision_event()
    }

    #[inline]
    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.impulse_joints.get(joint)
    }

    #[inline]
    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.impulse_joints.get_mut(joint)
    }

    pub fn component(&self, collider_handle: &ColliderHandle) -> Option<(ComponentTypeId, ComponentHandle)> {
        self.component_mapping.get(collider_handle).cloned()
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
