/// Shura version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod component;
mod data;
mod graphics;
mod input;
mod math;
mod scene;
mod shura;
mod world;

pub use instant::{Duration, Instant};
pub use rustc_hash::{FxHashMap, FxHashSet};
pub use shura_proc::*;

#[cfg(target_os = "android")]
pub use ::winit::platform::android::activity::AndroidApp;

pub(crate) use {component::controller_manager::*, data::arena::*};

pub use crate::{
    component::{
        component::*, component_config::*, component_handle::*, component_manager::*,
        component_set::*, component_type::*, empty_component::*, group::*, position_component::*,
    },
    graphics::{
        camera::*, color::*, frame_manager::*, gpu::*, instance_buffer::*, model::*,
        render_encoder::*, render_target::*, renderer::*, screen_config::*, shader::*, sprite::*,
        sprite_sheet::*, uniform::*, vertex::*,
    },
    input::input::{Input, InputEvent, InputTrigger, Key, Modifier, MouseButton, ScreenTouch},
    math::{aabb::*, math::*},
    scene::{context::*, scene::*, scene_manager::*},
    shura::*,
};

/// Access to [wgpu](https://github.com/gfx-rs/wgpu) for creating custom graphics.
pub use wgpu;

/// Access to [winit](https://github.com/rust-windowing/winit).
pub use winit;

// Rodio
#[cfg(feature = "audio")]
mod sound;
#[cfg(feature = "audio")]
/// Access to [rodio](https://github.com/RustAudio/rodio)
pub mod audio {
    pub use crate::sound::audio_manager::*;
    pub use crate::sound::sound::*;
    pub use rodio::Sink as AudioSink;
    pub use rodio::*;
}

pub use bytemuck;
/// Access to [image](https://github.com/image-rs/image)
pub use image;

#[cfg(not(feature = "physics"))]
pub use world::world_no_rapier::World;

#[cfg(feature = "physics")]
pub use world::world::World;

#[cfg(feature = "physics")]
/// Access to the to [rapier2d](https://github.com/dimforge/rapier)
pub mod physics {
    pub use crate::world::{
        // character_controller_component::*,
        collider_component::*,
        rigid_body_component::*,
        world::CollideType,
    };
    pub use rapier2d::control::{
        CharacterAutostep, CharacterCollision, CharacterLength, EffectiveCharacterMovement,
        KinematicCharacterController,
    };
    pub use rapier2d::geometry::*;
    pub use rapier2d::parry;
    pub use rapier2d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ActiveHooks, CoefficientCombineRule, Collider,
        ColliderBroadPhaseData, ColliderBuilder, ColliderChanges, ColliderFlags, ColliderHandle,
        ColliderMaterial, ColliderParent, ColliderSet, ColliderShape, ColliderType, FixedJoint,
        FixedJointBuilder, GenericJoint, GenericJointBuilder, Group, ImpulseJoint,
        ImpulseJointHandle, InteractionGroups, LockedAxes, MassProperties, MotorModel,
        PrismaticJoint, QueryFilter, QueryFilterFlags, Ray, RayIntersection, RevoluteJoint,
        RevoluteJointBuilder, RigidBody, RigidBodyActivation, RigidBodyBuilder, RigidBodyHandle,
        RigidBodySet, RigidBodyType, Shape, ShapeType, SharedShape, SpacialVector, TypedShape, TOI,
    };
    pub mod rapier {
        pub use rapier2d::*;
    }
}

// egui
#[cfg(feature = "gui")]
/// Access to [egui](https://github.com/emilk/egui)
pub mod gui {
    pub(crate) use crate::graphics::gui::gui::*;
    pub use egui::Context as GuiContext;
    pub use egui::*;
}

// serde
#[cfg(feature = "serde")]
pub mod serde {
    pub use crate::scene::scene_serde::*;
    pub use bincode;
    pub use serde::*;
}

// text
#[cfg(feature = "text")]
/// Text rendering inspired by [wgpu_text](https://github.com/Blatko1/wgpu-text)
pub mod text {
    pub use crate::graphics::text::text::*;
}

// gamepad
#[cfg(feature = "gamepad")]
/// Access to [gilrs](https://gitlab.com/gilrs-project/gilrs)
pub mod gamepad {
    pub use crate::input::input::{GamepadButton, GamepadStick};
    pub use gilrs::{
        ev, ff, Axis, Button, ConnectedGamepadsIterator, Gamepad, GamepadId, Mapping, MappingError,
        MappingSource, PowerInfo,
    };
}

// animation
#[cfg(feature = "animation")]
mod tween;

/// Access to animations inspired by [bevy_tweening](https://github.com/djeedai/bevy_tweening)
#[cfg(feature = "animation")]
pub mod animation {
    pub use crate::tween::{ease::*, tween::*};
}

/// Access to [nalgebra](https://github.com/dimforge/nalgebra), the math library used by shura
pub use nalgebra;

/// Access to [rayon](https://github.com/rayon-rs/rayon)
#[cfg(feature = "rayon")]
pub use rayon;

/// Access to [mint](https://github.com/kvark/mint) to convert between the diffrent math types
pub use mint;

/// Access to some easy randomizer functions
pub mod rand {
    pub fn gen_range<
        T: distributions::uniform::SampleUniform,
        R: distributions::uniform::SampleRange<T>,
    >(
        range: R,
    ) -> T {
        return thread_rng().gen_range(range);
    }
    pub fn gen_bool(p: f64) -> bool {
        return thread_rng().gen_bool(p);
    }

    pub use rand::*;
}

#[cfg(feature = "log")]
mod logging;

#[cfg(feature = "log")]
/// Access to the logging abstraction over [env_logger](https://github.com/rust-cli/env_logger) and modified version of [wasm_logger](https://gitlab.com/limira-rs/wasm-logger)
pub mod log {
    pub use crate::logging::logging::LoggerBuilder;
    pub use env_logger;
    pub use log::{debug, error, info, trace, warn, Level, LevelFilter, SetLoggerError};
}
