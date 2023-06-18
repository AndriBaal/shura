use crate::Camera;
use crate::Vector;
#[cfg(feature = "gamepad")]
use gilrs::*;
use instant::{Duration, Instant};
use rustc_hash::FxHashMap;
use winit::event::*;

#[cfg(feature = "log")]
use crate::log::info;

pub use winit::event::ModifiersState as Modifier;
pub use winit::event::MouseButton;
pub use winit::event::VirtualKeyCode as Key;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
/// Indicates if the screen is touched anywhere.
pub struct ScreenTouch;

#[cfg(feature = "gamepad")]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
/// Button on a GamePad
pub struct GamepadButton {
    pub gamepad: GamepadId,
    pub button: Button,
}

#[cfg(feature = "gamepad")]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum GamepadStick {
    Left,
    Right,
}

impl GamepadButton {
    pub fn new(gamepad: GamepadId, button: Button) -> Self {
        Self { gamepad, button }
    }
}

#[cfg(feature = "gamepad")]
impl From<GamepadButton> for InputTrigger {
    fn from(k: GamepadButton) -> Self {
        Self::GamepadButton(k)
    }
}

impl From<Key> for InputTrigger {
    fn from(k: Key) -> Self {
        Self::Key(k)
    }
}

impl From<MouseButton> for InputTrigger {
    fn from(m: MouseButton) -> Self {
        Self::MouseButton(m)
    }
}

impl From<ScreenTouch> for InputTrigger {
    fn from(t: ScreenTouch) -> Self {
        Self::ScreenTouch(t)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
/// Trigger of a key-like event
pub enum InputTrigger {
    Key(Key),
    MouseButton(MouseButton),
    ScreenTouch(ScreenTouch),
    #[cfg(feature = "gamepad")]
    GamepadButton(GamepadButton),
}

/// Event of a [InputTrigger] that holds the trigger and the time of the event.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct InputEvent {
    trigger: InputTrigger,
    pressed: bool,
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "serde", serde(default = "Instant::now"))]
    start: Instant,
    frames: u32,
    pressure: f32,
}

impl InputEvent {
    pub fn new(trigger: InputTrigger, pressure: f32) -> Self {
        Self {
            trigger,
            pressed: true,
            start: Instant::now(),
            frames: 1,
            pressure,
        }
    }

    // Value between 0 and 1 of how much a analog key is pressed
    pub fn pressure(&self) -> f32 {
        self.pressure
    }

    pub fn update(&mut self) {
        self.pressed = false;
        self.frames += 1;
    }

    pub fn frames(&self) -> u32 {
        self.frames
    }

    pub fn held_time(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    pub fn trigger(&self) -> InputTrigger {
        self.trigger
    }
}

/// Manage input from touch devices, keyboards, mice and gamepads.
pub struct Input {
    cursor_raw: Vector<u32>,
    touches: FxHashMap<u64, Vector<u32>>,
    events: FxHashMap<InputTrigger, InputEvent>,
    modifiers: Modifier,
    wheel_delta: f32,
    window_size: Vector<f32>,
    #[cfg(feature = "gamepad")]
    game_pad_manager: Gilrs,
    #[cfg(feature = "gamepad")]
    active_gamepad: Option<GamepadId>,
    #[cfg(feature = "gamepad")]
    dead_zone: f32,
}

impl Input {
    #[cfg(feature = "gamepad")]
    pub const DEFAULT_DEAD_ZONE: f32 = 0.1;

    pub(crate) fn new(window_size: Vector<u32>) -> Self {
        Self {
            cursor_raw: Vector::new(0, 0),
            touches: Default::default(),
            events: Default::default(),
            modifiers: Default::default(),
            wheel_delta: 0.0,
            window_size: window_size.cast(),
            #[cfg(feature = "gamepad")]
            game_pad_manager: match Gilrs::new() {
                Ok(ok) => ok,
                Err(err) => match err {
                    Error::NotImplemented(gilrs) => gilrs,
                    Error::InvalidAxisToBtn => panic!("Gamepad Error: Invalid Axis To Button!"),
                    Error::Other(err) => panic!("Gamepad Error: {}", err),
                },
            },
            #[cfg(feature = "gamepad")]
            active_gamepad: None,
            #[cfg(feature = "gamepad")]
            dead_zone: Self::DEFAULT_DEAD_ZONE,
        }
    }

    pub(crate) fn resize(&mut self, window_size: Vector<u32>) {
        self.window_size = window_size.cast()
    }

    pub(crate) fn on_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_raw = Vector::new(position.x as u32, position.y as u32);
            }
            WindowEvent::Touch(touch) => {
                let pos = Vector::new(touch.location.x as u32, touch.location.y as u32);
                self.cursor_raw = pos;
                match touch.phase {
                    TouchPhase::Started => {
                        let trigger = ScreenTouch.into();
                        self.touches.insert(touch.id, pos);
                        self.events.insert(trigger, InputEvent::new(trigger, 1.0));
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        let trigger = ScreenTouch.into();
                        self.touches.remove(&touch.id);
                        self.events.remove(&trigger);
                    }
                    TouchPhase::Moved => {
                        if let Some(touch) = self.touches.get_mut(&touch.id) {
                            *touch = pos;
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                Some(key) => {
                    let trigger = key.into();
                    match input.state {
                        ElementState::Pressed => {
                            if !self.events.contains_key(&trigger) {
                                self.events.insert(trigger, InputEvent::new(trigger, 1.0));
                            }
                        }
                        ElementState::Released => {
                            self.events.remove(&trigger);
                        }
                    }
                }
                None => {}
            },
            WindowEvent::MouseInput { state, button, .. } => {
                let trigger = (*button).into();
                match state {
                    ElementState::Pressed => {
                        self.events.insert(trigger, InputEvent::new(trigger, 1.0));
                    }
                    ElementState::Released => {
                        self.events.remove(&trigger);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_x, y) => {
                    self.wheel_delta = *y;
                    // if *y > 0.0 {
                    //     let trigger =
                    //     self.events.insert(trigger, InputEvent::new(trigger));
                    // } else if *y < 0.0 {
                    //     let trigger = key.into();
                    //     self.events.insert(trigger, InputEvent::new(trigger));
                    // }
                }
                MouseScrollDelta::PixelDelta(delta) => {
                    self.wheel_delta = if delta.y > 0.0 { 1.0 } else { -1.0 };
                }
            },
            WindowEvent::ModifiersChanged(state) => {
                self.modifiers = *state;
            }
            _ => {}
        }
    }

    pub(crate) fn update(&mut self) {
        self.wheel_delta = 0.0;

        for trigger in self.events.values_mut() {
            trigger.update();
        }
    }

    pub fn are_pressed(&self, trigger: &[impl Into<InputTrigger> + Copy]) -> bool {
        trigger
            .iter()
            .all(|trigger| match self.events.get(&(*trigger).into()) {
                Some(i) => return i.is_pressed(),
                None => false,
            })
    }

    pub fn are_held(&self, trigger: &[impl Into<InputTrigger> + Copy]) -> bool {
        trigger
            .iter()
            .all(|trigger| self.events.contains_key(&(*trigger).into()))
    }

    pub fn any_pressed(&self, trigger: &[impl Into<InputTrigger> + Copy]) -> bool {
        trigger
            .iter()
            .any(|trigger| match self.events.get(&(*trigger).into()) {
                Some(i) => return i.is_pressed(),
                None => false,
            })
    }

    pub fn any_held(&self, trigger: &[impl Into<InputTrigger> + Copy]) -> bool {
        trigger
            .iter()
            .any(|trigger| self.events.contains_key(&(*trigger).into()))
    }

    pub fn events(&self) -> impl Iterator<Item = (&InputTrigger, &InputEvent)> {
        self.events.iter()
    }

    pub fn event(&self, trigger: impl Into<InputTrigger>) -> Option<&InputEvent> {
        self.events.get(&trigger.into())
    }

    pub fn is_pressed(&self, trigger: impl Into<InputTrigger>) -> bool {
        match self.events.get(&trigger.into()) {
            Some(i) => return i.is_pressed(),
            None => false,
        }
    }

    pub fn is_held(&self, trigger: impl Into<InputTrigger>) -> bool {
        return self.events.contains_key(&trigger.into());
    }

    pub fn held_time(&self, trigger: impl Into<InputTrigger>) -> f32 {
        match self.events.get(&trigger.into()) {
            Some(i) => return i.held_time().as_secs_f32(),
            None => 0.0,
        }
    }

    pub fn held_time_duration(&self, trigger: impl Into<InputTrigger>) -> Option<Duration> {
        match self.events.get(&trigger.into()) {
            Some(i) => return Some(i.held_time()),
            None => None,
        }
    }

    #[cfg(feature = "gamepad")]
    // Syncs the gamepad inputs to the inputs of the gamepad. This is automatically done once every update cycle.
    pub fn sync_gamepad(&mut self) {
        while let Some(event) = self.game_pad_manager.next_event() {
            let gamepad = event.id;
            self.active_gamepad = Some(gamepad);
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    let trigger = GamepadButton { gamepad, button };
                    self.events
                        .insert(trigger.into(), InputEvent::new(trigger.into(), 1.0));
                }
                EventType::ButtonChanged(button, pressure, _) => {
                    let trigger = GamepadButton { gamepad, button };
                    if pressure == 0.0 {
                        self.events.remove(&trigger.into());
                    } else {
                        self.events
                            .insert(trigger.into(), InputEvent::new(trigger.into(), pressure));
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    let trigger = GamepadButton { gamepad, button };
                    self.events.remove(&trigger.into());
                }
                EventType::Disconnected => {
                    #[cfg(feature = "log")]
                    {
                        info!("Dropped gamepad: {}", gamepad);
                    }
                    self.events.retain(|trigger, _| match trigger {
                        InputTrigger::GamepadButton(c) => c.gamepad != gamepad,
                        _ => true,
                    })
                }
                // TODO: Maybe support this
                EventType::ButtonRepeated(_, _) => {}
                EventType::Dropped => {}
                EventType::Connected => {
                    #[cfg(feature = "log")]
                    {
                        let gamepad_ref = self.gamepad(gamepad).unwrap();
                        info!(
                            "Connected gamepad: {} with power {:?} and id {}",
                            gamepad_ref.name(),
                            gamepad_ref.power_info(),
                            gamepad
                        );
                    }
                }
                EventType::AxisChanged(_, _, _) => {}
            }
        }
        if let Some(active) = self.active_gamepad {
            let exists = self.game_pad_manager.gamepad(active).is_connected();
            if !exists {
                self.active_gamepad = None;
            }
        }
    }

    #[cfg(feature = "gamepad")]
    /// Returns a vector between [-1.0, -1.0] and [1.0, 1.0]
    pub fn gamepad_stick_deadzone(
        &self,
        gamepad_id: GamepadId,
        stick: GamepadStick,
        dead_zone: f32,
    ) -> Vector<f32> {
        fn axis_values(
            gamepad: &Gamepad,
            deadzone: f32,
            x_axis: Axis,
            y_axis: Axis,
        ) -> Vector<f32> {
            fn axis(gamepad: &Gamepad, x_axis: Axis) -> f32 {
                let value = gamepad.axis_data(x_axis).map(|a| a.value()).unwrap_or(0.0);
                return value;
            }
            let value = Vector::new(axis(gamepad, x_axis), axis(gamepad, y_axis));
            if value.magnitude() >= deadzone {
                return value;
            }
            return Vector::default();
        }
        if let Some(gamepad) = self.gamepad(gamepad_id) {
            match stick {
                GamepadStick::Left => {
                    return axis_values(&gamepad, dead_zone, Axis::LeftStickX, Axis::LeftStickY);
                }
                GamepadStick::Right => {
                    return axis_values(&gamepad, dead_zone, Axis::RightStickX, Axis::RightStickY);
                }
            }
        }
        return Vector::default();
    }

    #[cfg(feature = "gamepad")]
    /// Returns a vector between [-1.0, -1.0] and [1.0, 1.0]
    pub fn gamepad_stick(&self, gamepad_id: GamepadId, stick: GamepadStick) -> Vector<f32> {
        return Self::gamepad_stick_deadzone(&self, gamepad_id, stick, self.dead_zone);
    }

    #[cfg(feature = "gamepad")]
    pub fn dead_zone(&self) -> f32 {
        return self.dead_zone;
    }

    #[cfg(feature = "gamepad")]
    pub fn set_dead_zone(&mut self, val: f32) {
        self.dead_zone = val;
    }

    #[cfg(feature = "gamepad")]
    pub fn active_gamepad(&self) -> Option<GamepadId> {
        return self.active_gamepad;
    }

    #[cfg(feature = "gamepad")]
    pub fn first_gamepad(&self) -> Option<(GamepadId, Gamepad)> {
        return self.game_pad_manager.gamepads().next();
    }

    #[cfg(feature = "gamepad")]
    pub fn gamepads(&self) -> ConnectedGamepadsIterator {
        return self.game_pad_manager.gamepads();
    }

    #[cfg(feature = "gamepad")]
    pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
        return self.game_pad_manager.connected_gamepad(gamepad_id);
    }

    pub const fn modifiers(&self) -> Modifier {
        self.modifiers
    }

    pub const fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }

    pub const fn cursor_raw(&self) -> &Vector<u32> {
        &self.cursor_raw
    }

    pub fn touches_raw(&self) -> impl Iterator<Item = (&u64, &Vector<u32>)> {
        self.touches.iter()
    }

    pub fn compute_cursor(&self, cursor: Vector<u32>, camera: &Camera) -> Vector<f32> {
        let fov = camera.fov() * 2.0;
        let camera_translation = camera.translation();
        let cursor: Vector<f32> = Vector::new(cursor.x as f32, cursor.y as f32);
        camera_translation
            + Vector::new(
                cursor.x / self.window_size.x * fov.x - fov.x / 2.0,
                cursor.y / self.window_size.y * -fov.y + fov.y / 2.0,
            )
    }

    pub fn cursor(&self, camera: &Camera) -> Vector<f32> {
        self.compute_cursor(self.cursor_raw, camera)
    }

    pub fn touches(&self, camera: &Camera) -> Vec<(u64, Vector<f32>)> {
        let mut touches = vec![];
        for (id, raw) in &self.touches {
            touches.push((*id, self.compute_cursor(*raw, camera)));
        }
        return touches;
    }

    #[cfg(feature = "gamepad")]
    pub fn set_gamepad_mapping(
        &mut self,
        gamepad_id: GamepadId,
        mapping: &Mapping,
        name: Option<&str>,
    ) -> Result<String, MappingError> {
        return self
            .game_pad_manager
            .set_mapping(gamepad_id.into(), mapping, name);
    }

    #[cfg(feature = "gamepad")]
    pub fn set_gamepad_mapping_strict(
        &mut self,
        gamepad_id: GamepadId,
        mapping: &Mapping,
        name: Option<&str>,
    ) -> Result<String, MappingError> {
        return self
            .game_pad_manager
            .set_mapping_strict(gamepad_id.into(), mapping, name);
    }
}
