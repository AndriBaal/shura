mod input;

pub use input::*;
pub use gilrs::{
    ev, ff, Axis, Button, ConnectedGamepadsIterator, Gamepad, GamepadId, Mapping, MappingError,
    MappingSource, PowerInfo,
};
