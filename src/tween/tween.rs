use crate::{animation::EaseMethod, Duration, Isometry, Rotation, Vector};

// Animations heavily inspired by bevy_tweening

pub trait Stepable: Copy + Clone {
    fn step(&mut self, end: &Self, factor: f32) -> Self;
}

impl Stepable for Isometry<f32> {
    fn step(&mut self, end: &Self, factor: f32) -> Self {
        self.lerp_slerp(end, factor)
    }
}

impl Stepable for f32 {
    fn step(&mut self, end: &Self, factor: f32) -> Self {
        *self * (1.0 - factor) + end * factor
    }
}

impl Stepable for Vector<f32> {
    fn step(&mut self, end: &Self, factor: f32) -> Self {
        self.lerp(end, factor)
    }
}

impl Stepable for Rotation<f32> {
    fn step(&mut self, end: &Self, factor: f32) -> Self {
        self.slerp(end, factor)
    }
}

pub trait Tweenable {
    type Output: Stepable;
    fn value(&self) -> &Self::Output;
    fn duration(&self) -> Duration;
    fn total_duration(&self) -> TotalDuration;
    fn set_elapsed(&mut self, elapsed: Duration);
    fn elapsed(&self) -> Duration;
    fn tick(&mut self, delta: Duration) -> TweenState;
    fn rewind(&mut self);
    fn set_progress(&mut self, progress: f32) {
        self.set_elapsed(self.duration().mul_f32(progress.max(0.)));
    }
    fn progress(&self) -> f32 {
        let elapsed = self.elapsed();
        if let TotalDuration::Finite(total_duration) = self.total_duration() {
            if elapsed >= total_duration {
                return 1.;
            }
        }
        (elapsed.as_secs_f64() / self.duration().as_secs_f64()).fract() as f32
    }
    fn times_completed(&self) -> u32 {
        (self.elapsed().as_nanos() / self.duration().as_nanos()) as u32
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TweenSequence<T: Stepable> {
    tweens: Vec<Tween<T>>,
    index: usize,
    duration: Duration,
    elapsed: Duration,
    value: T,
}

impl<T: Stepable> TweenSequence<T> {
    pub fn new(items: impl IntoIterator<Item = Tween<T>>) -> Self {
        let tweens: Vec<_> = items.into_iter().collect();
        assert!(!tweens.is_empty());
        let duration = tweens.iter().map(Tweenable::duration).sum();
        Self {
            value: tweens.first().unwrap().start,
            tweens,
            index: 0,
            duration,
            elapsed: Duration::ZERO,
        }
    }

    #[must_use]
    pub fn then(mut self, tween: Tween<T>) -> Self {
        self.duration += tween.duration();
        self.tweens.push(tween);
        self
    }

    #[must_use]
    pub fn index(&self) -> usize {
        self.index.min(self.tweens.len() - 1)
    }

    #[must_use]
    pub fn current(&self) -> &Tween<T> {
        &self.tweens[self.index()]
    }
}

impl<T: Stepable> Tweenable for TweenSequence<T> {
    type Output = T;
    fn duration(&self) -> Duration {
        self.duration
    }

    fn total_duration(&self) -> TotalDuration {
        TotalDuration::Finite(self.duration)
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        // Set the total sequence progress
        self.elapsed = elapsed;

        // Find which tween is active in the sequence
        let mut accum_duration = Duration::ZERO;
        for index in 0..self.tweens.len() {
            let tween = &mut self.tweens[index];
            let tween_duration = tween.duration();
            if elapsed < accum_duration + tween_duration {
                self.index = index;
                let local_duration = elapsed - accum_duration;
                tween.set_elapsed(local_duration);
                // TODO?? set progress of other tweens after that one to 0. ??
                return;
            }
            tween.set_elapsed(tween.duration()); // ?? to prepare for next loop/rewind?
            accum_duration += tween_duration;
        }

        // None found; sequence ended
        self.index = self.tweens.len();
    }

    fn elapsed(&self) -> Duration {
        self.elapsed
    }

    fn tick(&mut self, mut delta: Duration) -> TweenState {
        self.elapsed = self.elapsed.saturating_add(delta).min(self.duration);
        while self.index < self.tweens.len() {
            let tween = &mut self.tweens[self.index];
            let tween_remaining = tween.duration() - tween.elapsed();
            if let TweenState::Active = tween.tick(delta) {
                self.value = tween.value;
                return TweenState::Active;
            }

            tween.rewind();
            delta -= tween_remaining;
            self.index += 1;
        }

        TweenState::Completed
    }

    fn rewind(&mut self) {
        self.elapsed = Duration::ZERO;
        self.index = 0;
        for tween in &mut self.tweens {
            // or only first?
            tween.rewind();
        }
    }

    fn value(&self) -> &T {
        &self.value
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tween<T: Stepable> {
    ease_function: EaseMethod,
    elapsed: Duration,
    duration: Duration,
    total_duration: TotalDuration,
    strategy: RepeatStrategy,
    direction: TweeningDirection,
    start: T,
    end: T,
    value: T,
}

impl<T: Stepable> Tween<T> {
    pub fn new(ease_function: impl Into<EaseMethod>, duration: Duration, start: T, end: T) -> Self {
        Self {
            ease_function: ease_function.into(),

            elapsed: Duration::ZERO,
            duration,
            total_duration: compute_total_duration(duration, RepeatCount::default()),
            strategy: RepeatStrategy::default(),
            direction: TweeningDirection::Forward,
            start,
            end,
            value: start,
        }
    }

    #[must_use]
    pub fn direction(&self) -> TweeningDirection {
        self.direction
    }

    #[must_use]
    pub fn repeat_strategy(&self) -> RepeatStrategy {
        self.strategy
    }

    #[must_use]
    pub fn with_repeat_count(mut self, count: impl Into<RepeatCount>) -> Self {
        self.total_duration = compute_total_duration(self.duration, count.into());
        self
    }

    #[must_use]
    pub fn with_repeat_strategy(mut self, strategy: RepeatStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    #[must_use]
    pub fn with_direction(mut self, direction: TweeningDirection) -> Self {
        self.direction = direction;
        self
    }

    #[must_use]
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.set_progress(progress);
        self
    }

    pub fn set_repeat_count(&mut self, count: impl Into<RepeatCount>) {
        self.total_duration = compute_total_duration(self.duration, count.into());
    }

    pub fn set_repeat_strategy(&mut self, strategy: RepeatStrategy) {
        self.strategy = strategy;
    }

    pub fn set_direction(&mut self, direction: TweeningDirection) {
        self.direction = direction;
    }

    pub fn start(&self) -> &T {
        &self.start
    }

    pub fn end(&self) -> &T {
        &self.end
    }

    // Clock

    fn tick_clock(&mut self, tick: Duration) -> (TweenState, i32) {
        self.set_elapsed(self.elapsed.saturating_add(tick))
    }

    pub fn times_completed(&self) -> u32 {
        (self.elapsed.as_nanos() / self.duration.as_nanos()) as u32
    }

    fn set_elapsed(&mut self, elapsed: Duration) -> (TweenState, i32) {
        let old_times_completed = self.times_completed();

        self.elapsed = elapsed;

        let state = match self.total_duration {
            TotalDuration::Finite(total_duration) => {
                if self.elapsed >= total_duration {
                    self.elapsed = total_duration;
                    TweenState::Completed
                } else {
                    TweenState::Active
                }
            }
            TotalDuration::Infinite => TweenState::Active,
        };

        (
            state,
            self.times_completed() as i32 - old_times_completed as i32,
        )
    }

    fn elapsed(&self) -> Duration {
        self.elapsed
    }

    fn state(&self) -> TweenState {
        match self.total_duration {
            TotalDuration::Finite(total_duration) => {
                if self.elapsed >= total_duration {
                    TweenState::Completed
                } else {
                    TweenState::Active
                }
            }
            TotalDuration::Infinite => TweenState::Active,
        }
    }

    fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
    }
}

impl<T: Stepable> Tweenable for Tween<T> {
    type Output = T;
    fn value(&self) -> &T {
        &self.value
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn total_duration(&self) -> TotalDuration {
        self.total_duration
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        self.set_elapsed(elapsed);
    }

    fn elapsed(&self) -> Duration {
        self.elapsed()
    }

    fn tick(&mut self, delta: Duration) -> TweenState {
        if self.state() == TweenState::Completed {
            return TweenState::Completed;
        }

        // Tick the animation clock
        let (state, times_completed) = self.tick_clock(delta);
        let (progress, times_completed_for_direction) = match state {
            TweenState::Active => (self.progress(), times_completed),
            TweenState::Completed => (1., times_completed.max(1) - 1), // ignore last
        };
        if self.strategy == RepeatStrategy::MirroredRepeat && times_completed_for_direction & 1 != 0
        {
            self.direction = !self.direction;
        }

        // Apply the lens, even if the animation finished, to ensure the state is
        // consistent
        let mut factor = progress;
        if self.direction == TweeningDirection::Backward {
            factor = 1. - factor;
        }
        let factor = self.ease_function.sample(factor);

        self.value = self.start.step(&self.end, factor);

        state
    }

    fn rewind(&mut self) {
        if self.strategy == RepeatStrategy::MirroredRepeat {
            // In mirrored mode, direction alternates each loop. To reset to the original
            // direction on Tween creation, we count the number of completions, ignoring the
            // last one if the Tween is currently in TweenState::Completed because that one
            // freezes all parameters.
            let mut times_completed = self.times_completed();
            if self.state() == TweenState::Completed {
                debug_assert!(times_completed > 0);
                times_completed -= 1;
            }
            if times_completed & 1 != 0 {
                self.direction = !self.direction;
            }
        }
        self.reset();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TweeningDirection {
    Forward,
    Backward,
}

impl Default for TweeningDirection {
    fn default() -> Self {
        Self::Forward
    }
}

impl std::ops::Not for TweeningDirection {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TweenState {
    Active,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TotalDuration {
    Finite(Duration),
    Infinite,
}

fn compute_total_duration(duration: Duration, count: RepeatCount) -> TotalDuration {
    match count {
        RepeatCount::Finite(times) => TotalDuration::Finite(duration.saturating_mul(times)),
        RepeatCount::For(duration) => TotalDuration::Finite(duration),
        RepeatCount::Infinite => TotalDuration::Infinite,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RepeatStrategy {
    /// Reset the animation back to its starting position.
    Repeat,
    /// Follow a ping-pong pattern, changing the direction each time an endpoint
    /// is reached.
    ///
    /// A complete cycle start -> end -> start always counts as 2 loop
    /// iterations for the various operations where looping matters. That
    /// is, a 1 second animation will take 2 seconds to end up back where it
    /// started.
    MirroredRepeat,
}

impl Default for RepeatStrategy {
    fn default() -> Self {
        Self::Repeat
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RepeatCount {
    /// Run the animation N times.
    Finite(u32),
    /// Run the animation for some amount of time.
    For(Duration),
    /// Loop the animation indefinitely.
    Infinite,
}

impl Default for RepeatCount {
    fn default() -> Self {
        Self::Finite(1)
    }
}

impl From<u32> for RepeatCount {
    fn from(value: u32) -> Self {
        Self::Finite(value)
    }
}

impl From<Duration> for RepeatCount {
    fn from(value: Duration) -> Self {
        Self::For(value)
    }
}
