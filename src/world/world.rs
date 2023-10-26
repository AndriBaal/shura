use crate::{ComponentHandle, FrameManager, ComponentTypeId, Component};
use rapier2d::{crossbeam, prelude::*};
use rayon::iter::plumbing::ProducerCallback;
use rustc_hash::FxHashMap;

type EventReceiver<T> = crossbeam::channel::Receiver<T>;
type ColliderMapping = FxHashMap<ColliderHandle, ComponentHandle>;
type RigidBodyMapping = FxHashMap<RigidBodyHandle, ComponentHandle>;

struct WorldEvents {
    collision: EventReceiver<CollisionEvent>,
    contact_force: EventReceiver<ContactForceEvent>,
    collector: ChannelEventCollector,
}

impl Default for WorldEvents {
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

pub type CollisionCallback = fn(c1: (ComponentHandle, ColliderHandle), c2: (ComponentHandle, ColliderHandle), ty: CollideType);
#[derive(Default)]
pub struct WorldHooks {
    collisions: FxHashMap<(ComponentTypeId, ComponentTypeId), CollisionCallback>
}

impl WorldHooks {
    pub fn collision<C1: Component, C2: Component>(&mut self, callback: CollisionCallback) {
        let key = if C1::IDENTIFIER < C2::IDENTIFIER {
            (C1::IDENTIFIER, C2::IDENTIFIER)
        } else {
            (C2::IDENTIFIER, C1::IDENTIFIER)
        };
        self.collisions.insert(key, callback);
    }
}


#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum CollideType {
    Started,
    Stopped,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct World {
    pub time_scale: f32,
    pub gravity: Vector<f32>,
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
    events: WorldEvents,
}

impl Clone for World {
    fn clone(&self) -> Self {
        Self {
            bodies: self.bodies.clone(),
            colliders: self.colliders.clone(),
            collider_mapping: self.collider_mapping.clone(),
            rigid_body_mapping: self.rigid_body_mapping.clone(),
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
            collider_mapping: Default::default(),
            rigid_body_mapping: Default::default(),
            gravity: vector![0.0, 0.0],
            time_scale: 1.0,
        }
    }

    pub(crate) fn add_collider(
        &mut self,
        component_handle: ComponentHandle,
        collider: Collider,
    ) -> ColliderHandle {
        let collider_handle = self.colliders.insert(collider.clone());
        self.collider_mapping
            .insert(collider_handle, component_handle);
        return collider_handle;
    }

    pub(crate) fn add_rigid_body(
        &mut self,
        rigid_body: RigidBody,
        colliders: Vec<Collider>,
        component_handle: ComponentHandle,
    ) -> RigidBodyHandle {
        let rigid_body_handle = self.bodies.insert(rigid_body.clone());
        self.rigid_body_mapping
            .insert(rigid_body_handle, component_handle);
        for collider in colliders {
            let collider_handle = self.colliders.insert_with_parent(
                collider.clone(),
                rigid_body_handle,
                &mut self.bodies,
            );
            self.collider_mapping
                .insert(collider_handle, component_handle);
        }
        return rigid_body_handle;
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
        return None;
    }

    pub(crate) fn remove_collider(&mut self, collider: ColliderHandle) -> Option<Collider> {
        self.collider_mapping.remove(&collider);
        if let Some(collider) =
            self.colliders
                .remove(collider, &mut self.islands, &mut self.bodies, false)
        {
            return Some(collider);
        }
        return None;
    }

    pub(crate) fn attach_collider(
        &mut self,
        rigid_body_handle: RigidBodyHandle,
        collider: impl Into<Collider>,
    ) -> Option<ColliderHandle> {
        if let Some(component) = self.rigid_body_mapping.get(&rigid_body_handle) {
            let collider =
                self.colliders
                    .insert_with_parent(collider, rigid_body_handle, &mut self.bodies);
            self.collider_mapping.insert(collider, *component);
            return Some(collider);
        }
        return None;
    }

    pub(crate) fn detach_collider(&mut self, collider_handle: ColliderHandle) -> Option<Collider> {
        self.collider_mapping.remove(&collider_handle);
        let collider =
            self.colliders
                .remove(collider_handle, &mut self.islands, &mut self.bodies, true);
        return collider;
    }

    pub fn step(&mut self, frame: &FrameManager, hooks: WorldHooks) {
        self.integration_parameters.dt = frame.frame_time() * self.time_scale;
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
            &self.events.collector,
        );
        self.query_pipeline.update(&self.bodies, &self.colliders);
        
        macro_rules! skip_fail {
            ($res:expr) => {
                match $res {
                    Some(val) => val,
                    None => {
                        continue;
                    }
                }
            };
        }
        while let Ok(collision_event) = self.events.collision.try_recv() {
            let collision_type = if collision_event.started() {
                CollideType::Started
            } else {
                CollideType::Stopped
            };
            let collider_handle1 = collision_event.collider1();
            let collider_handle2 = collision_event.collider2();
            let component1 = *skip_fail!(self.component_from_collider(&collider_handle1));
            let component2 = *skip_fail!(self.component_from_collider(&collider_handle2));

            let (key, params) = if component1.component_type_id() < component2.component_type_id() {
                ((component1.component_type_id(), component2.component_type_id()), ((component1, collider_handle1), (component2, collider_handle2)))
            } else {
                ((component2.component_type_id(), component1.component_type_id()), ((component2, collider_handle2), (component1, collider_handle1)))
            };

            if let Some(callback) = hooks.collisions.get(&key) {
                (callback)(params.0, params.1, collision_type);
            }
        }
    }

    #[cfg(feature = "serde")]
    pub(crate) fn remove_no_maintain(&mut self, position: &dyn crate::Position) {
        use crate::physics::{
            ColliderComponent, ColliderStatus, RigidBodyComponent, RigidBodyStatus,
        };
        if let Some(component) = position.downcast_ref::<RigidBodyComponent>() {
            match component.status {
                RigidBodyStatus::Added { rigid_body_handle } => {
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
                RigidBodyStatus::Pending { .. } => return,
            }
        } else if let Some(component) = position.downcast_ref::<ColliderComponent>() {
            match component.status {
                ColliderStatus::Added { collider_handle } => {
                    self.collider_mapping.remove(&collider_handle);
                    self.colliders.remove(
                        collider_handle,
                        &mut self.islands,
                        &mut self.bodies,
                        false,
                    );
                }
                ColliderStatus::Pending { .. } => return,
            }
        }
    }

    // pub(crate) fn move_shape(
    //     &self,
    //     controller: &mut CharacterControllerComponent,
    //     frame: &FrameManager,
    //     bodies: &RigidBodySet,
    //     colliders: &ColliderSet,
    //     queries: &QueryPipeline,
    //     character_shape: &dyn Shape,
    //     character_pos: &Isometry<Real>,
    //     desired_translation: Vector<Real>,
    //     filter: QueryFilter,
    //     mut events: impl FnMut(CharacterCollision, &World),
    // ) -> EffectiveCharacterMovement {

    // }

    pub fn component_from_collider(
        &self,
        collider_handle: &ColliderHandle,
    ) -> Option<&ComponentHandle> {
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
    ) -> Option<(ComponentHandle, ColliderHandle, f32)> {
        if let Some(collider) =
            self.query_pipeline
                .cast_ray(&self.bodies, &self.colliders, ray, max_toi, solid, filter)
        {
            if let Some(component) = self.component_from_collider(&collider.0) {
                return Some((*component, collider.0, collider.1));
            }
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
            if let Some(component) = self.component_from_collider(&collider.0) {
                return Some((*component, collider.0, collider.1));
            }
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
            if let Some(component) = self.component_from_collider(&collider.0) {
                return Some((*component, collider.0, collider.1));
            }
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
                if let Some(component) = self.component_from_collider(&collider) {
                    return callback(*component, collider, ray);
                }
                return true;
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
            |collider| {
                if let Some(component) = self.component_from_collider(&collider) {
                    return callback(*component, collider);
                }
                return true;
            },
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
            if let Some(component) = self.component_from_collider(&collider) {
                return Some((*component, collider));
            }
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
            |collider| {
                if let Some(component) = self.component_from_collider(&collider) {
                    return callback(*component, collider);
                }
                return true;
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

    pub fn joint(&self, joint: ImpulseJointHandle) -> Option<&ImpulseJoint> {
        self.impulse_joints.get(joint)
    }

    pub fn joint_mut(&mut self, joint: ImpulseJointHandle) -> Option<&mut ImpulseJoint> {
        self.impulse_joints.get_mut(joint)
    }

    pub fn gravity(&self) -> Vector<f32> {
        self.gravity
    }

    pub fn time_scale(&self) -> f32 {
        self.time_scale
    }

    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.gravity = gravity;
    }

    pub fn set_time_scale(&mut self, time_scale: f32) {
        self.time_scale = time_scale;
    }
}
