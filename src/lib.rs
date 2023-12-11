/// Shura version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "animation")]
pub mod animation;
pub mod app;
#[cfg(feature = "audio")]
pub mod audio;
pub mod component;
pub mod context;
pub mod data;
pub mod entity;
pub mod graphics;
#[cfg(feature = "gui")]
pub mod gui;
pub mod input;
#[cfg(feature = "log")]
pub mod log;
pub mod math;
#[cfg(feature = "physics")]
pub mod physics;
pub mod rand;
pub mod resource;
pub mod scene;
#[cfg(feature = "serde")]
pub mod serde;
pub mod system;
pub mod tasks;
#[cfg(feature = "text")]
pub mod text;
pub mod time;

pub use bytemuck;
pub use image;
pub use instant;
pub use mint;
pub use nalgebra as na;
#[cfg(feature = "rayon")]
pub use rayon;
pub use rustc_hash;
pub use shura_proc as macros;
pub use wgpu;
pub use winit;

#[cfg(target_arch = "wasm32")]
pub use web_sys;

#[cfg(target_arch = "wasm32")]
pub use reqwest;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures;

pub use crate::macros::main;

pub mod prelude {
    pub use crate::macros::main;

    #[cfg(feature = "animation")]
    pub use crate::animation::*;
    pub use crate::app::*;
    #[cfg(feature = "audio")]
    pub use crate::audio::*;
    pub use crate::component::*;
    pub use crate::context::*;
    pub use crate::data::*;
    pub use crate::entity::*;
    pub use crate::graphics::*;
    #[cfg(feature = "gui")]
    pub use crate::gui;
    pub use crate::input::*;
    #[cfg(feature = "log")]
    pub use crate::log::*;
    pub use crate::macros::*;
    pub use crate::math::*;
    #[cfg(feature = "physics")]
    pub use crate::physics;
    pub use crate::rand::*;
    pub use crate::resource::*;
    pub use crate::scene::*;
    #[cfg(feature = "serde")]
    pub use crate::serde::*;
    pub use crate::system::*;
    pub use crate::tasks::*;
    #[cfg(feature = "text")]
    pub use crate::text::*;
    pub use crate::time::*;

    pub use bytemuck;
    pub use image;
    pub use instant;
    pub use mint;
    pub use nalgebra as na;
    pub use rayon;
    pub use rustc_hash;
    pub use shura_proc as macros;
    pub use wgpu;
    pub use winit;

    #[cfg(feature = "rayon")]
    pub use rayon::prelude::ParallelIterator;

    #[cfg(target_arch = "wasm32")]
    pub use web_sys;

    #[cfg(target_arch = "wasm32")]
    pub use reqwest;

    #[cfg(target_arch = "wasm32")]
    pub use wasm_bindgen_futures;
}

// pub(crate) use data::arena::*;

// #[cfg(not(feature = "physics"))]
// pub use physics::world_no_rapier::World;

// #[cfg(feature = "physics")]
// pub use physics::world::World;

// #[cfg(feature = "physics")]
// /// Access to the to [rapier2d](https://github.com/dimforge/rapier)
// pub mod physics {
//     pub use crate::physics::{
//         collider_component::*,
//         rigid_body_component::*,
//         // character_controller_component::*,
//         world::*,
//     };
//     pub use rapier2d::control::{
//         CharacterAutostep, CharacterCollision, CharacterLength, EffectiveCharacterMovement,
//         KinematicCharacterController,
//     };
//     pub use rapier2d::geometry::*;
//     pub use rapier2d::parry;
//     pub use rapier2d::prelude::{
//         ActiveCollisionTypes, ActiveEvents, ActiveHooks, CoefficientCombineRule, Collider,
//         ColliderBroadPhaseData, ColliderBuilder, ColliderChanges, ColliderFlags, ColliderHandle,
//         ColliderMaterial, ColliderParent, ColliderSet, ColliderShape, ColliderType, FixedJoint,
//         FixedJointBuilder, GenericJoint, GenericJointBuilder, Group, ImpulseJoint,
//         ImpulseJointHandle, InteractionGroups, LockedAxes, MassProperties, MotorModel,
//         PrismaticJoint, QueryFilter, QueryFilterFlags, Ray, RayIntersection, RevoluteJoint,
//         RevoluteJointBuilder, RigidBody, RigidBodyActivation, RigidBodyBuilder, RigidBodyHandle,
//         RigidBodySet, RigidBodyType, Shape, ShapeType, SharedShape, SpacialVector, TypedShape, TOI,
//     };
//     pub mod rapier {
//         pub use rapier2d::*;
//     }
// }
