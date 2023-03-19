use crate::Camera;
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
/// Trigger of a key-like event
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
    cycles: u32,
}

impl InputEvent {
    pub fn new(trigger: InputTrigger) -> Self {
        Self {
            trigger,
            pressed: true,
            start: Instant::now(),
            cycles: 1,
        }
    }

    pub fn update(&mut self) {
        self.pressed = false;
        self.cycles += 1;
    }

    pub fn cycles(&self) -> u32 {
        self.cycles
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
    #[cfg(feature = "gamepad")]
    game_pad_manager: Option<Gilrs>,
}

impl Input {
    pub(crate) fn new() -> Self {
        Self {
            cursor_raw: Vector::new(0, 0),
            touches: Default::default(),
            events: Default::default(),
            modifiers: Default::default(),
            wheel_delta: 0.0,
            #[cfg(feature = "gamepad")]
            game_pad_manager: Gilrs::new().ok(),
        }
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
                        self.events.insert(trigger, InputEvent::new(trigger));
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
                                self.events.insert(trigger, InputEvent::new(trigger));
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
                        self.events.insert(trigger, InputEvent::new(trigger));
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
    pub fn gamepads(&self) -> Option<ConnectedGamepadsIterator> {
        if let Some(game_pad_manager) = &self.game_pad_manager {
            return Some(game_pad_manager.gamepads());
        }
        return None;
    }

    #[cfg(feature = "gamepad")]
    pub fn gamepad(&self, gamepad_id: GamepadId) -> Option<Gamepad> {
        if let Some(game_pad_manager) = &self.game_pad_manager {
            return game_pad_manager.connected_gamepad(gamepad_id);
        }
        return None;
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

    pub fn compute_cursor(
        &self,
        window_size: Vector<u32>,
        cursor: Vector<u32>,
        camera: &Camera,
    ) -> Vector<f32> {
        let fov = camera.fov();
        let camera_translation = camera.translation();
        let window_size = Vector::new(window_size.x as f32, window_size.y as f32);
        let cursor: Vector<f32> = Vector::new(cursor.x as f32, cursor.y as f32);
        camera_translation
            + Vector::new(
                cursor.x / window_size.x * fov.x - fov.x / 2.0,
                cursor.y / window_size.y * -fov.y + fov.y / 2.0,
            )
    }

    pub fn cursor_camera(&self, window_size: Vector<u32>, camera: &Camera) -> Vector<f32> {
        self.compute_cursor(window_size, self.cursor_raw, camera)
    }

    pub fn touches_camera(
        &self,
        window_size: Vector<u32>,
        camera: &Camera,
    ) -> Vec<(u64, Vector<f32>)> {
        let mut touches = vec![];
        for (id, raw) in &self.touches {
            touches.push((*id, self.compute_cursor(window_size, *raw, camera)));
        }
        return touches;
    }

    pub fn events(&self) -> impl Iterator<Item = (&InputTrigger, &InputEvent)> {
        self.events.iter()
    }

    pub fn event(&self, trigger: impl Into<InputTrigger>) -> Option<&InputEvent> {
        self.events.get(&trigger.into())
    }

    // Setters

    //
    // #[cfg(feature = "gamepad")]
    // pub fn set_gamepad_mapping(
    //     &mut self,
    //     gamepad_id: GamepadId,
    //     mapping: &Mapping,
    //     name: Option<&str>,
    // ) -> Result<String, MappingError> {
    //     if let Some(game_pad_manager) = &mut self.game_pad_manager {
    //         return game_pad_manager.set_mapping(gamepad_id.into(), mapping, name);
    //     }
    //     return Err(MappingError::NotConnected);
    // }

    //
    // #[cfg(feature = "gamepad")]
    // pub fn set_gamepad_mapping_strict(
    //     &mut self,
    //     gamepad_id: GamepadId,
    //     mapping: &Mapping,
    //     name: Option<&str>,
    // ) -> Result<String, MappingError> {
    //     if let Some(game_pad_manager) = &mut self.game_pad_manager {
    //         return game_pad_manager.set_mapping_strict(gamepad_id.into(), mapping, name);
    //     }
    //     return Err(MappingError::NotConnected);
    // }
}
