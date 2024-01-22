pub use crate::time::{Duration, Instant};
#[cfg(feature = "log")]
use log::info;

pub struct TimeManager {
    delta_time: Duration,
    total_time: Duration,
    last_time: Duration,
    fps_time: Duration,
    start_time: Instant,
    update_time: Instant,
    total_frames: u64,
    fps_counter: u32,
    fps: u32,
}

impl TimeManager {
    pub const MAX_FRAME_TIME: Duration = Duration::from_millis(50);
    pub(crate) fn new() -> Self {
        let now = Instant::now();
        let elapsed = now.elapsed();
        Self {
            delta_time: elapsed,
            last_time: elapsed,
            total_time: elapsed,
            start_time: now,
            update_time: now,
            fps_time: elapsed,
            total_frames: 0,
            fps_counter: 0,
            fps: 0,
        }
    }

    pub(crate) fn tick(&mut self) {
        self.update_time = Instant::now();
        self.total_time = self.update_time - self.start_time;

        self.fps_counter += 1;
        self.total_frames += 1;
        let new_frame_time = self.total_time - self.last_time;

        if new_frame_time > Self::MAX_FRAME_TIME {
            self.delta_time = Self::MAX_FRAME_TIME;
        } else {
            self.delta_time = new_frame_time;
        }

        if self.total_time > self.fps_time + Duration::from_secs(1) {
            self.fps = self.fps_counter;
            self.fps_time = self.total_time;
            self.fps_counter = 0;
            #[cfg(feature = "log")]
            {
                info!("fps: {}\tdelta: {}", self.fps, self.delta());
                #[cfg(feature = "rayon")]
                info!("threads: {}", rayon::current_num_threads());
            }
        }

        self.last_time = self.total_time;
    }

    pub const fn start(&self) -> Instant {
        self.start_time
    }

    pub const fn update(&self) -> Instant {
        self.update_time
    }

    pub fn now(&self) -> Instant {
        Instant::now()
    }

    pub fn delta(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    pub fn total(&self) -> f32 {
        self.total_time.as_secs_f32()
    }

    pub const fn frame_time_duration(&self) -> Duration {
        self.delta_time
    }

    pub const fn frames_since_last_seconds(&self) -> u32 {
        self.fps_counter
    }

    pub const fn total_time_duration(&self) -> Duration {
        self.total_time
    }

    pub const fn total_frames(&self) -> u64 {
        self.total_frames
    }

    pub const fn fps(&self) -> u32 {
        self.fps
    }
}
