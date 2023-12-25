mod input;

#[cfg(feature = "gamepad")]
pub use gilrs::{
    ev, ff, Axis, Button, ConnectedGamepadsIterator, Gamepad, GamepadId, Mapping, MappingError,
    MappingSource, PowerInfo,
};
pub use input::*;
