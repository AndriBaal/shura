#[cfg(feature = "physics")]
mod simple_character_controller_component;
#[cfg(feature = "physics")]
mod collider_component;
mod component;
mod position_component;
#[cfg(feature = "physics")]
mod rigid_body_component;

#[cfg(feature = "physics")]
pub use simple_character_controller_component::*;
#[cfg(feature = "physics")]
pub use collider_component::*;
pub use component::*;
pub use position_component::*;
#[cfg(feature = "physics")]
pub use rigid_body_component::*;
