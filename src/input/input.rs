use crate::Vector;
#[cfg(feature = "gamepad")]
use gilrs::*;
use instant::{Duration, Instant};
use rustc_hash::FxHashMap;
use winit::event::*;

pub use winit::event::ModifiersState as Modifier;
pub use winit::event::MouseButton;
pub use winit::event::VirtualKeyCode as Key;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// Indicates if the screen is touched anywhere.
pub struct ScreenTouch;

// #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
// pub enum Scroll {
//     Up,
//     Down
// }

// impl From<Scroll> for InputTrigger {
//     fn from(s: Scroll) -> Self {
//         Self::Scroll(s)
//     }
// }

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
/// Trigger of a key-like event. Gamepad inputs can be get with the (gamepad)[crate::Context::gamepad] method.
pub enum InputTrigger {
    Key(Key),
    MouseButton(MouseButton),
    ScreenTouch(ScreenTouch),
}

/// Event of a [InputTrigger] that holds the trigger and the time of the event.
pub struct InputEvent {
    trigger: InputTrigger,
    pressed: bool,
    start: Instant,
}

impl InputEvent {
    pub fn new(trigger: InputTrigger) -> Self {
        Self {
            trigger,
            pressed: true,
            start: Instant::now(),
        }
    }

    pub fn normalize(&mut self) {
        self.pressed = false;
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
    cursor_raw: Vector<f32>,
    touches: FxHashMap<u64, Vector<f32>>,
    triggers: FxHashMap<InputTrigger, InputEvent>,
    modifiers: Option<Modifier>,
    wheel_delta: f32,
    staged_key: Option<Key>,
    #[cfg(feature = "gamepad")]
    game_pad_manager: Option<Gilrs>,
}

impl Input {
    pub(crate) fn new() -> Self {
        Self {
            cursor_raw: Vector::new(0.0, 0.0),
            touches: Default::default(),
            triggers: Default::default(),
            modifiers: None,
            staged_key: None,
            wheel_delta: 0.0,
            #[cfg(feature = "gamepad")]
            game_pad_manager: Gilrs::new().ok(),
        }
    }

    pub(crate) fn event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_raw = Vector::new(position.x as f32, position.y as f32);
            }
            WindowEvent::Touch(touch) => {
                let pos = Vector::new(touch.location.x as f32, touch.location.y as f32);
                self.cursor_raw = pos;
                match touch.phase {
                    TouchPhase::Started => {
                        let trigger = ScreenTouch.into();
                        self.touches.insert(touch.id, pos);
                        self.triggers.insert(trigger, InputEvent::new(trigger));
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        let trigger = ScreenTouch.into();
                        self.touches.remove(&touch.id);
                        self.triggers.remove(&trigger);
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
                            if !self.triggers.contains_key(&trigger) {
                                self.triggers.insert(trigger, InputEvent::new(trigger));
                            }
                        }
                        ElementState::Released => {
                            self.triggers.remove(&trigger);
                        }
                    }
                }
                None => {}
            },
            WindowEvent::MouseInput { state, button, .. } => {
                let trigger = (*button).into();
                match state {
                    ElementState::Pressed => {
                        self.triggers.insert(trigger, InputEvent::new(trigger));
                    }
                    ElementState::Released => {
                        self.triggers.remove(&trigger);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_x, y) => {
                    self.wheel_delta = *y;
                    // if *y > 0.0 {
                    //     let trigger =
                    //     self.triggers.insert(trigger, InputEvent::new(trigger));
                    // } else if *y < 0.0 {
                    //     let trigger = key.into();
                    //     self.triggers.insert(trigger, InputEvent::new(trigger));
                    // }
                }
                MouseScrollDelta::PixelDelta(delta) => {
                    self.wheel_delta = if delta.y > 0.0 { 1.0 } else { -1.0 };
                }
            },
            WindowEvent::ModifiersChanged(state) => {
                self.modifiers = Some(*state);
            }
            _ => {}
        }
    }

    pub(crate) fn update(&mut self) {
        self.wheel_delta = 0.0;
        self.modifiers = None;
        self.staged_key = None;

        for trigger in self.triggers.values_mut() {
            trigger.normalize();
        }
    }

    #[inline]
    pub fn is_pressed(&self, trigger: impl Into<InputTrigger>) -> bool {
        match self.triggers.get(&trigger.into()) {
            Some(i) => return i.is_pressed(),
            None => false,
        }
    }

    #[inline]
    pub fn is_held(&self, trigger: impl Into<InputTrigger>) -> bool {
        return self.triggers.contains_key(&trigger.into());
    }

    #[inline]
    pub fn held_time(&self, trigger: impl Into<InputTrigger>) -> f32 {
        match self.triggers.get(&trigger.into()) {
            Some(i) => return i.held_time().as_secs_f32(),
            None => 0.0,
        }
    }

    #[inline]
    pub fn held_time_duration(&self, trigger: impl Into<InputTrigger>) -> Option<Duration> {
        match self.triggers.get(&trigger.into()) {
            Some(i) => return Some(i.held_time()),
            None => None,
        }
    }

    // Getters

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn gamepads(&self) -> Option<ConnectedGamepadsIterator> {
        if let Some(game_pad_manager) = &self.game_pad_manager {
            return Some(game_pad_manager.gamepads());
        }
        return None;
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
        if let Some(game_pad_manager) = &self.game_pad_manager {
            return game_pad_manager.connected_gamepad(gamepad_id);
        }
        return None;
    }

    #[inline]
    pub const fn touches(&self) -> &FxHashMap<u64, Vector<f32>> {
        &self.touches
    }

    #[inline]
    pub const fn staged_key(&self) -> Option<Key> {
        self.staged_key
    }

    #[inline]
    pub const fn triggers(&self) -> &FxHashMap<InputTrigger, InputEvent> {
        &self.triggers
    }

    #[inline]
    pub const fn modifiers(&self) -> Option<Modifier> {
        self.modifiers
    }

    #[inline]
    pub const fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }

    #[inline]
    pub const fn cursor_raw(&self) -> &Vector<f32> {
        &self.cursor_raw
    }

    // Setters

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn set_mapping(
        &mut self,
        gamepad_id: GamepadId,
        mapping: &Mapping,
        name: Option<&str>,
    ) -> Result<String, MappingError> {
        if let Some(game_pad_manager) = &mut self.game_pad_manager {
            return game_pad_manager.set_mapping(gamepad_id.into(), mapping, name);
        }
        return Err(MappingError::NotConnected);
    }

    #[inline]
    #[cfg(feature = "gamepad")]
    pub fn set_mapping_strict(
        &mut self,
        gamepad_id: GamepadId,
        mapping: &Mapping,
        name: Option<&str>,
    ) -> Result<String, MappingError> {
        if let Some(game_pad_manager) = &mut self.game_pad_manager {
            return game_pad_manager.set_mapping_strict(gamepad_id.into(), mapping, name);
        }
        return Err(MappingError::NotConnected);
    }
}
