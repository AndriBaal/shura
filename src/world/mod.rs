#[cfg(feature = "physics")]
pub mod collider_component;
#[cfg(feature = "physics")]
pub mod rigid_body_component;
// TODO: Implement character controller when top down functionality is provided
// pub mod character_controller_component;
#[cfg(feature = "physics")]
pub mod world;
#[cfg(not(feature = "physics"))]
pub mod world_no_rapier;
