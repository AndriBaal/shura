// TODO: Doc

#![crate_type = "lib"]
#![crate_name = "shura"]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod component;
mod data;
mod graphics;
mod input;
mod math;
mod scene;
mod shura;
mod state;

pub use instant::Duration;
pub use rustc_hash::{FxHashMap, FxHashSet};
pub use shura_proc::*;

#[cfg(target_os = "android")]
pub use ::winit::platform::android::activity::AndroidApp;

pub(crate) use {
    component::controller_caller::*, data::arena::*, data::arena_path::*,
    scene::context::ShuraFields,
};

pub use crate::{
    component::{
        base_component::*, component_config::*, component_derive::*, component_group::*,
        component_handle::*, component_manager::*, component_set::*, component_type::*,
    },
    graphics::{
        camera::*, color::*, frame_manager::*, gpu::*, instance_buffer::*, model::*,
        render_encoder::*, render_target::*, renderer::*, screen_config::*, shader::*, sprite::*,
        sprite_sheet::*, uniform::*, vertex::*,
    },
    input::input::*,
    math::{math::*, matrix::*},
    scene::{context::Context, scene::*, scene_manager::*},
    shura::*,
    state::{
        global_state::*,
        scene_state::{SceneStateController, SceneStateManager},
        state::*,
    },
};

/// Access to [wgpu](https://github.com/gfx-rs/wgpu) for creating custom graphics.
pub mod wgpu {
    pub use wgpu::*;
}

/// Access to [winit](https://github.com/rust-windowing/winit).
pub mod winit {
    pub use winit::*;
}

// Rodio
#[cfg(feature = "audio")]
mod sound;
#[cfg(feature = "audio")]
/// Access to [rodio](https://github.com/RustAudio/rodio)
pub mod audio {
    pub use crate::sound::sound::*;
    pub use rodio::*;
}

// Rapier2d
#[cfg(feature = "physics")]
mod world;
#[cfg(feature = "physics")]
/// Access to the to [rapier2d](https://github.com/dimforge/rapier)
pub mod physics {
    pub use crate::world::world::CollideType;
    pub use crate::world::world::{RcWorld, World};
    pub use rapier2d::geometry::*;
    pub use rapier2d::parry;
    pub use rapier2d::prelude::*;
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

// text
#[cfg(feature = "text")]
/// Abstraction of [wgpu_glyph](https://github.com/hecrj/wgpu_glyph) to render text onto [sprites](crate::Sprite).
pub mod text {
    pub use crate::graphics::text::{font::*, text::*};
}

// gamepad
#[cfg(feature = "gamepad")]
/// Access to [gilrs](https://gitlab.com/gilrs-project/gilrs)
pub mod gamepad {
    pub use gilrs::{
        ev, ff, Axis, Button, ConnectedGamepadsIterator, Gamepad, GamepadId, Mapping, MappingError,
        MappingSource, PowerInfo,
    };
}

// serde
#[cfg(feature = "serde")]
pub use crate::scene::scene_serde::*;

// animation
#[cfg(feature = "animation")]
mod tween;
/// Access to animations
#[cfg(feature = "animation")]
pub mod animation {
    pub use crate::tween::{ease::*, tween::*};
}

/// Access to [nalgebra](https://github.com/dimforge/nalgebra), the math library used by shura
pub mod na {
    pub use nalgebra::*;
}

/// Access to [mint](https://github.com/kvark/mint) to convert between the diffrent math types
pub mod mint {
    pub use mint::*;
}

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
    pub use log::{debug, error, info, trace, warn, Level, LevelFilter, SetLoggerError};
    pub mod env_logger {
        pub use env_logger::*;
    }
}
