//! shura - A safe 2D game engine to easily create manageable games
//!
//! shura is a safe, fast and cross-platform 2D component-based game framework written in rust. shura helps you to manage big games with a component system, scene managing and its group system.
//! The main goal of shura is, that your games logic can be separated into different components, groups and scenes where the logic is easily manageable and safe to control.
//!
//! Here are some main features of the engine:
//!
//! - Managing multiple independent scenes.
//!
//! - Easy to use component system with a group system to ensure fast manageable 2D games in massive levels
//!
//! - Group system that acts like a chunk system to organize components and manage big worlds
//!
//! - Built in support for postprocessing of your renders
//!
//! - Physics simulations directly implemented into the component system through rapier (feature flag 'physics')
//!
//! - Window Management with winit
//!
//! - Cross-platform extendable rendering with wgpu
//!
//! - Input handling for touch, mouse and keyboard and controller with gilrs (feature flag 'gamepad')
//!
//! - Text rendering with wgpu_glyph (feature flag 'text')
//!
//! - Audio playback with rodio (feature flag 'audio')
//!
//! - Easily create GUI's with egui(feature flag 'gui')
//!
//! Feedback is very welcome since shura is still in its beta phase.
//!

#![crate_type = "lib"]
#![crate_name = "shura"]

mod component;
mod data;
mod animation;
mod graphics;
mod input;
mod math;
mod scene;
mod shura_core;

pub use instant::Duration;
pub use shura_proc::*;

pub(crate) use {component::controller_caller::*, data::arena::*, data::arena_path::*};

pub use crate::{
    animation::animation::*,
    component::{
        base_component::*, component_config::*, component_derive::*, component_group::*,
        component_handle::*, component_manager::*, component_set::*, component_type::*,
    },
    graphics::{camera::*, frame_manager::*},
    graphics::{
        color::*, gpu::*, instance_buffer::*, model::*, render_encoder::*, render_taget::*,
        renderer::*, screen_config::*, shader::*, sprite::*, sprite_sheet::*, uniform::*,
        vertex::*,
    },
    input::{input::*},
    math::{math::*, matrix::*},
    scene::{context::*, scene::*, scene_manager::*},
    shura_core::*,
};

/// Access to [wgpu](https://github.com/gfx-rs/wgpu) for creating custom graphics.
pub mod wgpu {
    pub use wgpu::*;
}

/// Access to the windowing library [winit](https://github.com/rust-windowing/winit).
pub mod winit {
    pub use winit::*;
}

// Rodio
#[cfg(feature = "audio")]
mod sound;
#[cfg(feature = "audio")]
/// Access to [rodio](https://github.com/RustAudio/rodio) library
pub mod audio {
    pub use crate::sound::sound::*;
    pub use rodio::*;
}

// Rapier2d
#[cfg(feature = "physics")]
mod world;
#[cfg(feature = "physics")]
/// Access to the relevant items from the [rapier2d](https://github.com/dimforge/rapier) library.
pub mod physics {
    pub use crate::world::world::CollideType;
    pub(crate) use crate::world::world::World;
    pub use rapier2d::geometry::*;
    pub use rapier2d::parry;
    pub use rapier2d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ActiveHooks, CoefficientCombineRule, Collider,
        ColliderBroadPhaseData, ColliderBuilder, ColliderChanges, ColliderFlags, ColliderHandle,
        ColliderMaterial, ColliderParent, ColliderShape, ColliderType, FixedJoint,
        FixedJointBuilder, GenericJoint, GenericJointBuilder, Group, ImpulseJoint,
        ImpulseJointHandle, InteractionGroups, LockedAxes, MassProperties, MotorModel,
        PrismaticJoint, QueryFilter, QueryFilterFlags, Ray, RayIntersection, RevoluteJoint,
        RevoluteJointBuilder, RigidBody, RigidBodyActivation, RigidBodyBuilder, RigidBodyHandle,
        RigidBodyType, Shape, ShapeType, SharedShape, SpacialVector, TypedShape, TOI,
    };
}

// egui
#[cfg(feature = "gui")]
/// Access to [egui](https://github.com/emilk/egui) library.
pub mod gui {
    pub(crate) use crate::graphics::gui::gui::*;
    pub use crate::Context;
    pub use egui::Context as GuiContext;
    pub use egui::*;
}

// text
#[cfg(feature = "text")]
/// Abstraction of [wgpu_glyph](https://github.com/hecrj/wgpu_glyph) to render text onto [sprites](crate::Sprite).
pub mod text {
    pub use crate::graphics::text::{font::*, text::*};
}

// gamepad
#[cfg(feature = "gamepad")]
/// Access to [gilrs](https://gitlab.com/gilrs-project/gilrs) library.
pub mod gamepad {
    pub use gilrs::{
        ev, ff, Axis, Button, ConnectedGamepadsIterator, Gamepad, GamepadId, Mapping, MappingError,
        MappingSource, PowerInfo,
    };
}

// serde
#[cfg(feature = "serde")]
pub use crate::scene::scene_serde::*;

pub mod na {
    pub use nalgebra::*;
}

pub mod mint {
    pub use mint::*;
}

pub mod log {
    pub use log::{debug, error, info, trace, warn};
}
