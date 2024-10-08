/// Shura version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "animation")]
pub mod animation;
pub mod app;
pub mod arena;
#[cfg(feature = "audio")]
pub mod audio;
pub mod context;
pub mod ecs;
pub mod graphics;
#[cfg(feature = "gui")]
pub mod gui;
pub mod input;
pub mod io;
#[cfg(feature = "log")]
pub mod log;
pub mod math;
#[cfg(feature = "physics")]
pub mod physics;
pub mod random;
pub mod scene;
#[cfg(feature = "serde")]
pub mod serde;
pub mod tasks;
#[cfg(feature = "text")]
pub mod text;
pub mod time;

pub use bytemuck;
pub use image;
pub use instant;
pub use mint;
pub use nalgebra;
#[cfg(feature = "rayon")]
pub use rayon;
pub use rustc_hash;
pub use shura_macros as macros;
pub use wgpu;
pub use winit;

#[cfg(target_arch = "wasm32")]
pub use web_sys;

#[cfg(target_arch = "wasm32")]
pub use reqwest;

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures;

pub use crate::macros::app;


pub mod prelude {
    pub use crate::winit::window::Window;

    #[cfg(feature = "animation")]
    pub use crate::animation::*;
    pub use crate::app::*;
    pub use crate::arena::*;
    #[cfg(feature = "audio")]
    pub use crate::audio::*;
    pub use crate::context::*;
    pub use crate::ecs::*;
    pub use crate::graphics::*;
    #[cfg(feature = "gui")]
    pub use crate::gui;
    pub use crate::input::*;
    pub use crate::io::*;
    #[cfg(feature = "log")]
    pub use crate::log::*;
    pub use crate::macros::*;
    pub use crate::math::*;
    #[cfg(feature = "physics")]
    pub use crate::physics::*;
    pub use crate::random::*;
    pub use crate::scene::*;
    #[cfg(feature = "serde")]
    pub use crate::serde::*;
    pub use crate::tasks::*;
    #[cfg(feature = "text")]
    pub use crate::text::*;
    pub use crate::time::*;

    pub use bytemuck;
    pub use image;
    pub use instant;
    pub use mint;
    pub use nalgebra;
    #[cfg(feature = "rayon")]
    pub use rayon;
    pub use rustc_hash;
    pub use shura_macros as macros;
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
