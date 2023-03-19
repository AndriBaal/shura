use crate::{animation::EaseMethod, Duration, Isometry};

// Animations heavily inspired by bevy_tweening

pub trait Tweenable {
    fn value(&self) -> Isometry<f32>;
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
pub struct TweenSequence {
    tweens: Vec<Tween>,
    index: usize,
    duration: Duration,
    elapsed: Duration,
    value: Isometry<f32>,
}

impl TweenSequence {
    pub fn new(items: impl IntoIterator<Item = Tween>) -> Self {
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
    pub fn then(mut self, tween: Tween) -> Self {
        self.duration += tween.duration();
        self.tweens.push(tween);
        self
    }

    #[must_use]
    pub fn index(&self) -> usize {
        self.index.min(self.tweens.len() - 1)
    }

    #[must_use]
    pub fn current(&self) -> &Tween {
        &self.tweens[self.index()]
    }
}

impl Tweenable for TweenSequence {
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

    fn value(&self) -> Isometry<f32> {
        self.value
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Tween {
    ease_function: EaseMethod,
    elapsed: Duration,
    duration: Duration,
    total_duration: TotalDuration,
    strategy: RepeatStrategy,
    direction: TweeningDirection,
    start: Isometry<f32>,
    end: Isometry<f32>,
    value: Isometry<f32>,
}

impl Tween {
    pub fn new(
        ease_function: impl Into<EaseMethod>,
        duration: Duration,
        start: Isometry<f32>,
        end: Isometry<f32>,
    ) -> Self {
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

    pub fn start(&self) -> Isometry<f32> {
        self.start
    }

    pub fn end(&self) -> Isometry<f32> {
        self.end
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

impl Tweenable for Tween {
    fn value(&self) -> Isometry<f32> {
        self.value
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

        self.value = self.start.lerp_slerp(&self.end, factor);

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
