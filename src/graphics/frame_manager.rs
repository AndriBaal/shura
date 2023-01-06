use log::info;
use instant::{Instant, Duration};

/// Acces to various frame informations.
pub struct FrameManager {
    delta_time: Duration,
    total_time: Duration,
    last_time: Duration,
    start_time: Instant,

    fps_time: Duration,
    total_frames: u64,
    fps_counter: u32,
    fps: u32,
}

impl FrameManager {
    pub(crate) fn new() -> Self {
        let now = Instant::now();
        let elapsed = now.elapsed();
        Self {
            delta_time: elapsed,
            last_time: elapsed,
            total_time: elapsed,
            start_time: now,

            fps_time: elapsed,
            total_frames: 0,
            fps_counter: 0,
            fps: 0,
        }
    }

    pub(crate) fn update(&mut self) {
        const MAX_DELTA_TIME: Duration = Duration::from_millis(100);

        self.total_time = self.start_time.elapsed();

        self.fps_counter += 1;
        self.total_frames += 1;
        let new_delta_time = self.total_time - self.last_time;

        if new_delta_time > MAX_DELTA_TIME {
            self.delta_time = MAX_DELTA_TIME;
        } else {
            self.delta_time = new_delta_time;
        }

        if self.total_time > self.fps_time + Duration::from_secs(1) {
            self.fps = self.fps_counter;
            self.fps_time = self.total_time;
            self.fps_counter = 0;
            info!("fps: {}\tdelta: {}", self.fps, self.delta_time());
        }

        self.last_time = self.total_time;
    }


    // Getter
    #[inline]
    pub fn delta_time(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    #[inline]
    pub fn total_time(&self) -> f32 {
        self.total_time.as_secs_f32()
    }

    #[inline]
    pub const fn delta_time_duration(&self) -> Duration {
        self.delta_time
    }

    #[inline]
    pub const fn total_time_duration(&self) -> Duration {
        self.total_time
    }

    #[inline]
    pub const fn total_frames(&self) -> u64 {
        self.total_frames
    }

    #[inline]
    pub const fn fps(&self) -> u32 {
        self.fps
    }
}
