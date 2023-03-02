use crate::{Camera, Dimension, Input, Vector, RELATIVE_CAMERA_SIZE};

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Cursor positions of a touch somewhere in the window.
pub struct Touch {
    pub id: u64,
    // Raw Cursor in pixel coordinates
    pub cursor_raw: Vector<u32>,
    // Absolute position of the cursor in the world
    pub cursor_world: Vector<f32>,
    // Relative position of the cursor to the screen
    pub cursor_relative: Vector<f32>,
}

/// Managing of the mouse cursor or various touch events. Every scene has its own [CursorManager]
/// because the world coordinates of the cursor depends on the camera.
#[derive(Clone, Debug, Default)]
pub struct CursorManager {
    // Raw Cursor in pixel coordinates
    cursor_raw: Vector<u32>,
    // Absolute position of the cursor in the world
    cursor_world: Vector<f32>,
    // Relative position of the cursor to the screen
    cursor_relative: Vector<f32>,
    touches: Vec<Touch>,
}

impl CursorManager {
    pub(crate) fn new() -> Self {
        Self {
            cursor_raw: Vector::new(0, 0),
            cursor_world: Vector::new(0.0, 0.0),
            cursor_relative: Vector::new(0.0, 0.0),
            touches: Default::default(),
        }
    }

    pub(crate) fn compute(&mut self, camera: &Camera, window_size: &Dimension<u32>, input: &Input) {
        let fov = camera.fov();
        let camera_translation = camera.translation();
        let window_size = Dimension::new(window_size.width as f32, window_size.height as f32);
        self.cursor_raw = *input.cursor_raw();

        let cursor_raw: Vector<f32> =
            Vector::new(self.cursor_raw.x as f32, self.cursor_raw.y as f32);
        self.cursor_world = camera_translation
            + Vector::new(
                cursor_raw.x / window_size.width * fov.width - fov.width / 2.0,
                cursor_raw.y / window_size.height * -fov.height + fov.height / 2.0,
            );

        self.cursor_relative = Vector::new(
            cursor_raw.x / window_size.width * RELATIVE_CAMERA_SIZE - RELATIVE_CAMERA_SIZE / 2.0,
            cursor_raw.y / window_size.height * -RELATIVE_CAMERA_SIZE + RELATIVE_CAMERA_SIZE / 2.0,
        );

        self.touches.clear();
        for (id, raw) in input.touches() {
            let raw_touch: Vector<f32> = Vector::new(raw.x as f32, raw.y as f32);
            let world = camera_translation
                + Vector::new(
                    raw_touch.x / window_size.width * fov.width - fov.width / 2.0,
                    raw_touch.y / window_size.height * -fov.height + fov.height / 2.0,
                );
            let relative_touch_pos = Vector::new(
                raw_touch.x / window_size.width * RELATIVE_CAMERA_SIZE - RELATIVE_CAMERA_SIZE / 2.0,
                raw_touch.y / window_size.height * -RELATIVE_CAMERA_SIZE
                    + RELATIVE_CAMERA_SIZE / 2.0,
            );
            self.touches.push(Touch {
                id: *id,
                cursor_raw: *raw,
                cursor_relative: relative_touch_pos,
                cursor_world: world,
            });
        }
    }

    pub fn touches(&self) -> &[Touch] {
        &self.touches
    }

    pub const fn cursor_raw(&self) -> &Vector<u32> {
        &self.cursor_raw
    }

    pub const fn cursor_world(&self) -> &Vector<f32> {
        &self.cursor_world
    }

    pub const fn cursor_relative(&self) -> &Vector<f32> {
        &self.cursor_relative
    }
}
