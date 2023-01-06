use crate::{Dimension, Input, Isometry, Vector, RELATIVE_CAMERA_SIZE};

#[derive(Copy, Clone, Debug)]
/// Cursor positions of a touch somewhere in the window.
pub struct Touch {
    pub id: u64,
    pub cursor_raw: Vector<f32>,
    pub cursor_world: Vector<f32>,
    pub cursor_relative: Vector<f32>,
}

/// Managing of the mouse cursor or various touch events. Every scene has its own [CursorManager]
/// because the world coordinates of the cursor depends on the camera.
pub struct CursorManager {
    // Absolute position of the cursor in the world
    cursor_world: Vector<f32>,
    // Relative position of the cursor to the screen
    cursor_relative: Vector<f32>,
    touches: Vec<Touch>,
}

impl CursorManager {
    pub(crate) fn new() -> Self {
        Self {
            cursor_world: Vector::new(0.0, 0.0),
            cursor_relative: Vector::new(0.0, 0.0),
            touches: Default::default(),
        }
    }

    pub(crate) fn compute(
        &mut self,
        fov: &Dimension<f32>,
        window_size: &Dimension<u32>,
        camera_pos: &Isometry<f32>,
        input: &Input,
    ) {
        fn apply_camera(mut cursor: Vector<f32>, camera: &Isometry<f32>) -> Vector<f32> {
            let s = -camera.rotation.sin_angle();
            let c = camera.rotation.cos_angle();
            let translation = camera.translation.vector;
            cursor += translation;
            return Vector::new(
                translation.x + (cursor.x - translation.x) * c - (cursor.y - translation.y) * s,
                translation.y + (cursor.x - translation.x) * s + (cursor.y - translation.y) * c,
            );
        }
        let window_size = Dimension::new(window_size.width as f32, window_size.height as f32);
        let raw_cursor = input.cursor_raw();
        self.cursor_world = apply_camera(
            Vector::new(
                raw_cursor.x / window_size.width * fov.width * 2.0 - fov.width,
                raw_cursor.y / window_size.height * -fov.height * 2.0 + fov.height,
            ),
            camera_pos,
        );

        let ratio = window_size.width / window_size.height;
        self.cursor_relative = Vector::new(
            raw_cursor.x / window_size.width * RELATIVE_CAMERA_SIZE - RELATIVE_CAMERA_SIZE / 2.0,
            -(raw_cursor.y / window_size.height * RELATIVE_CAMERA_SIZE
                - RELATIVE_CAMERA_SIZE / 2.0),
        );

        self.touches.clear();
        for (id, raw_touch) in input.touches() {
            let world = apply_camera(
                Vector::new(
                    raw_touch.x / window_size.width * fov.width * 2.0 - fov.width,
                    raw_touch.y / window_size.height * -fov.height * 2.0 + fov.height,
                ),
                camera_pos,
            );
            let relative_touch_pos = Vector::new(
                raw_touch.x / window_size.width * (ratio * 2.0) - 1.0,
                -(raw_touch.y / window_size.height * 2.0 - 1.0),
            );
            self.touches.push(Touch {
                id: *id,
                cursor_raw: *raw_touch,
                cursor_relative: relative_touch_pos,
                cursor_world: world,
            });
        }
    }

    #[inline]
    pub fn touches(&self) -> &[Touch] {
        &self.touches
    }

    #[inline]
    pub const fn cursor_world(&self) -> &Vector<f32> {
        &self.cursor_world
    }

    #[inline]
    pub const fn cursor_relative(&self) -> &Vector<f32> {
        &self.cursor_relative
    }
}
