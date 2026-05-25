#![cfg_attr(not(test), no_std)]

//! A small crate with implementations of various time controls for turn-based
//! games like chess and go.

use core::{fmt, time::Duration};

/// A generic trait for time controls
///
/// There are many types of time control, but they all have a number of features
/// in common. In general, a player's clock counts down while it's their turn
/// and may expire.
///
/// A time control is driven by calling:
///
/// - `turn_start` at the start of a turn.
/// - `turn_spend` one or more times during the turn. The sum of durations
///    passed to this function represents the total amount of time spent thinking
///    during the turn.
/// - `turn_end` at the end of the turn.
///
/// When a clock expires, it remains in the expired state until it is reset.
pub trait TimeControl: fmt::Display {
    /// Reset this clock to its initial configuration
    fn reset(&mut self);

    /// Returns whether this clock has hit zero, i.e. whether `turn_spend` has
    /// returned `true`.
    fn is_expired(&self) -> bool;

    /// Return the maximum amount of time left to think during this turn, in
    /// other words the amount of time that would cause the clock to expire.
    /// The meaning of this value after calling `turn_end` but before calling
    /// `turn_start` depends on the particular clock, but between `turn_start`
    /// and `turn_end` always indicates the value that if passed to `turn_spend`
    /// would cause the clock to expire.
    ///
    /// Note that e.g. spending 10s less than this duration to think does not
    /// mean `max_remaining` will return 10s on the next turn.
    fn max_remaining(&self) -> Duration;

    /// Apply any clock adjustments for the start of this player's turn. If the
    /// clock is already expired, this cannot un-expire it.
    fn turn_start(&mut self) {}

    /// Record that this player has spent the given amount of time thinking.
    /// Returns `true` if the clock has expired, or `false` if there is still
    /// time remaining.
    fn turn_spend(&mut self, elapsed: Duration) -> bool;

    /// Apply any clock adjustments for the end of this player's turn. If the
    /// clock is already expired, this cannot un-expire it.
    fn turn_end(&mut self) {}

    /// A combination of `turn_start`, `turn_spend`, and `turn_end`, a
    /// convenience function for use cases that only need to call `turn_spend`
    /// once at the end of a turn.
    fn complete_turn(&mut self, elapsed: Duration) -> bool {
        self.turn_start();
        let expire = self.turn_spend(elapsed);
        self.turn_end();
        expire
    }
}

impl<T> TimeControl for T
where
    T: AsMut<dyn TimeControl> + AsRef<dyn TimeControl> + fmt::Display,
{
    fn reset(&mut self) {
        self.as_mut().reset();
    }

    fn is_expired(&self) -> bool {
        self.as_ref().is_expired()
    }

    fn max_remaining(&self) -> Duration {
        self.as_ref().max_remaining()
    }

    fn turn_start(&mut self) {
        self.as_mut().turn_start();
    }

    fn turn_spend(&mut self, elapsed: Duration) -> bool {
        self.as_mut().turn_spend(elapsed)
    }

    fn turn_end(&mut self) {
        self.as_mut().turn_end();
    }

    fn complete_turn(&mut self, elapsed: Duration) -> bool {
        self.as_mut().complete_turn(elapsed)
    }
}

// Simple delay
// -----------------------------------------------------------------------------

/// Configuration for a simple delay clock
///
/// A fixed delay is added to the start of each turn before the clock starts
/// counting down.
#[derive(Copy, Clone, Debug)]
pub struct SimpleDelayConfig {
    /// The amount of time on the clock at the start of the game.
    pub initial: Duration,

    /// The amount of time to wait before counting down.
    pub delay: Duration,
}

impl Into<SimpleDelayClock> for SimpleDelayConfig {
    fn into(self) -> SimpleDelayClock {
        SimpleDelayClock::new(self)
    }
}

/// A simple delay clock
///
/// See [`SimpleDelayConfig`] for more information.
#[derive(Copy, Clone, Debug)]
pub struct SimpleDelayClock {
    config: SimpleDelayConfig,
    delay: Duration,
    main: Duration,
}

impl SimpleDelayClock {
    /// Create a new simple delay clock with the given config.
    pub fn new(config: SimpleDelayConfig) -> SimpleDelayClock {
        SimpleDelayClock {
            config,
            delay: Duration::ZERO,
            main: config.initial,
        }
    }
}

impl TimeControl for SimpleDelayClock {
    fn reset(&mut self) {
        *self = Self::new(self.config);
    }

    fn is_expired(&self) -> bool {
        self.main.is_zero()
    }

    fn max_remaining(&self) -> Duration {
        self.main + self.delay
    }

    fn turn_start(&mut self) {
        self.delay = match self.is_expired() {
            true => Duration::ZERO,
            false => self.config.delay,
        };
    }

    fn turn_spend(&mut self, elapsed: Duration) -> bool {
        let delay_use = self.delay.min(elapsed);
        let main_use = elapsed - delay_use;

        self.delay = self.delay.saturating_sub(delay_use);
        self.main = self.main.saturating_sub(main_use);

        self.main.is_zero()
    }

    fn turn_end(&mut self) {
        self.delay = Duration::ZERO;
    }
}

impl fmt::Display for SimpleDelayClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", ShowCountdown::with_minutes(self.main))?;
        if !self.delay.is_zero() {
            write!(f, " ({})", ShowCountdown::without_minutes(self.delay))?;
        }
        Ok(())
    }
}

// Fischer clock
// -----------------------------------------------------------------------------

/// Configuration for a Fischer clock
///
/// A Fischer clock is a simple countdown that adds an amount of time to the
/// countdown at the end of a turn. For example, with an initial period of 10:00
/// and an increment of :10, a turn that took 2:00 would end with 8:10 on
/// the clock.
///
/// Note that the increment is only added *after* the turn ends: a turn starting
/// with 2:00 on the clock will expire after 2:00 regardless of the size of the
/// increment.
#[derive(Copy, Clone, Debug)]
pub struct FischerConfig {
    /// The amount of time on the clock at the start of the game.
    pub initial: Duration,

    /// The amount of time added to the clock after finishing a turn.
    pub increment: Duration,
}

impl Into<FischerClock> for FischerConfig {
    fn into(self) -> FischerClock {
        FischerClock::new(self)
    }
}

/// A Fischer clock
///
/// See [`FischerConfig`] for more details.
#[derive(Copy, Clone, Debug)]
pub struct FischerClock {
    config: FischerConfig,
    main: Duration,
}

impl FischerClock {
    /// Create a new Fischer clock with the given config.
    pub fn new(config: FischerConfig) -> FischerClock {
        FischerClock {
            config,
            main: config.initial,
        }
    }
}

impl TimeControl for FischerClock {
    fn reset(&mut self) {
        *self = Self::new(self.config);
    }

    fn is_expired(&self) -> bool {
        self.main.is_zero()
    }

    fn max_remaining(&self) -> Duration {
        self.main
    }

    fn turn_spend(&mut self, elapsed: Duration) -> bool {
        self.main = self.main.saturating_sub(elapsed);
        self.main.is_zero()
    }

    fn turn_end(&mut self) {
        if !self.main.is_zero() {
            self.main += self.config.increment;
        }
    }
}

impl fmt::Display for FischerClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", ShowCountdown::with_minutes(self.main))
    }
}

// Byo-yomi
// -----------------------------------------------------------------------------

/// Configuration for a byo-yomi clock
///
/// A game with byo-yomi time controls has an initial period, which simply
/// counts down, then one or more byo-yomi periods at the end which are only
/// spent if the period time is fully used. The clock expires when all
/// byo-yomi periods are spent.
///
/// For example, if using a 10:00 initial period and one :30 byo-yomi period,
/// once the player has spent a total of 10:00 thinking, the byo-yomi countdown
/// starts and the player must complete each turn within the :30 byo-yomi
/// period. If playing with three byo-yomi periods, the player can let the
/// byo-yomi countdown expire two times, and on the third time the clock
/// expires. With this config, the following table shows how much time is left
/// on the clock after spending various amounts of time per turn within a
/// single game:
///
/// | Elapsed | At end of turn |   After turn |
/// | ------: | -------------: | -----------: |
/// |         |                | 10:00 +3x30s |
/// |    2:00 |    8:00 +3x30s |  8:00 +3x30s |
/// |    7:00 |    1:00 +3x30s |  1:00 +3x30s |
/// |    1:20 |     10s +2x30s |        3x30s |
/// |    0:20 |     10s +2x30s |        3x30s |
/// |    1:20 |     10s +0x30s |        1x30s |
/// |    0:40 |       Expired! |     Expired! |
#[derive(Copy, Clone, Debug)]
pub struct ByoYomiConfig {
    /// The amount of time on the clock at the start of the game.
    pub initial: Duration,

    /// The duration of each byo-yomi period.
    pub period_time: Duration,

    /// The number of byo-yomi periods that can be spent before the clock expires.
    pub num_periods: usize,
}

impl Into<ByoYomiClock> for ByoYomiConfig {
    fn into(self) -> ByoYomiClock {
        ByoYomiClock::new(self)
    }
}

/// A byo-yomi clock
///
/// See [`ByoYomiConfig`] for more details.
#[derive(Copy, Clone, Debug)]
pub struct ByoYomiClock {
    config: ByoYomiConfig,
    main: Duration,
    period: Duration,
    unused_periods: usize,
}

// This data structure is slightly awkward because we have to represent the
// state during a turn, as well as the state between turns, and also the state
// when a clock has expired. The convention here is that `period` is only
// nonzero during a turn that is in byo-yomi. When the turn ends, the byo-yomi
// period is "returned" to `unused_periods`. Thus, between turns, `main` and
// `period` will both be zero for a player who is in byo-yomi.

impl ByoYomiClock {
    /// Create a new byo-yomi clock with the given config.
    pub fn new(config: ByoYomiConfig) -> ByoYomiClock {
        ByoYomiClock {
            config,
            main: config.initial,
            period: Duration::ZERO,
            unused_periods: config.num_periods,
        }
    }

    pub fn in_byo_yomi(&self) -> bool {
        self.main.is_zero()
    }
}

impl TimeControl for ByoYomiClock {
    fn reset(&mut self) {
        *self = Self::new(self.config);
    }

    fn is_expired(&self) -> bool {
        self.in_byo_yomi() && self.period.is_zero() && self.unused_periods == 0
    }

    fn max_remaining(&self) -> Duration {
        self.main + self.period + self.config.period_time * self.unused_periods as u32
    }

    fn turn_start(&mut self) {
        if self.in_byo_yomi() && self.period.is_zero() && self.unused_periods > 0 {
            self.unused_periods -= 1;
            self.period = self.config.period_time;
        }
    }

    fn turn_spend(&mut self, mut elapsed: Duration) -> bool {
        // spend main time. this is a no-op if our main time is zero
        let spend_main = elapsed.min(self.main);
        self.main -= spend_main;
        elapsed -= spend_main;

        // any remaining time in `elapsed` should be spent on byo-yomi periods
        while !elapsed.is_zero() && !self.max_remaining().is_zero() {
            let spend_period = elapsed.min(self.period);
            self.period -= spend_period;
            elapsed -= spend_period;

            // if this period is spent, start the next one if available
            if self.period.is_zero() && self.unused_periods > 0 {
                self.period = self.config.period_time;
                self.unused_periods -= 1;
            }
        }

        self.is_expired()
    }

    fn turn_end(&mut self) {
        if self.in_byo_yomi() && !self.period.is_zero() {
            // return the unspent byo-yomi period.
            self.period = Duration::ZERO;
            self.unused_periods += 1;
        }
    }
}

impl fmt::Display for ByoYomiClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.in_byo_yomi() {
            if self.period.is_zero() && self.unused_periods > 0 {
                write!(f, "----")?;
            } else {
                let x = ShowCountdown::without_minutes(self.period);
                write!(f, "{x}")?;
            }
        } else {
            let x = ShowCountdown::with_minutes(self.main);
            write!(f, "{x}")?;
        }
        let period = ShowCountdown::without_minutes(self.config.period_time);
        write!(f, " +{}x{}", self.unused_periods, period)?;
        Ok(())
    }
}

// TwoPlayer
// -----------------------------------------------------------------------------

/// Track time controls for two players
pub struct TwoPlayer<P1, P2 = P1> {
    p1_turn: Option<bool>,
    p1: P1,
    p2: P2,
}

impl<P1> TwoPlayer<P1, P1>
where
    P1: TimeControl,
{
    /// Create a new two-player time control from the given config or clock.
    pub fn new<T>(from: T) -> Self
    where
        T: Into<P1> + Clone,
    {
        Self::new_asymmetric(from.clone(), from)
    }
}

impl<P1, P2> TwoPlayer<P1, P2>
where
    P1: TimeControl,
    P2: TimeControl,
{
    /// Create a new asymmetric two-player time control from the given configs
    /// or clocks. Initially, the time control is stopped for both players.
    pub fn new_asymmetric<T1, T2>(p1: T1, p2: T2) -> Self
    where
        T1: Into<P1>,
        T2: Into<P2>,
    {
        TwoPlayer {
            p1_turn: None,
            p1: p1.into(),
            p2: p2.into(),
        }
    }

    /// Reset the time control to its initial configuration. After resetting,
    /// the time control will be stopped for both players.
    pub fn reset(&mut self) {
        self.p1_turn = None;
        self.p1.reset();
        self.p2.reset();
    }

    /// Return whether the first player's clock is counting down.
    pub fn p1_turn(&self) -> bool {
        self.p1_turn.unwrap_or(false)
    }

    /// Return whether the second player's clock is counting down.
    pub fn p2_turn(&self) -> bool {
        !self.p1_turn.unwrap_or(true)
    }

    /// Return a reference to the first player's time control
    pub fn p1(&self) -> &P1 {
        &self.p1
    }

    /// Return a reference to the second player's time control
    pub fn p2(&self) -> &P2 {
        &self.p2
    }

    /// Return a mutable reference to the first player's time control
    pub fn p1_mut(&mut self) -> &mut P1 {
        &mut self.p1
    }

    /// Return a mutable reference to the second player's time control
    pub fn p2_mut(&mut self) -> &mut P2 {
        &mut self.p2
    }

    fn set_turn(&mut self, p1: Option<bool>) {
        let p1_turn = self.p1_turn();
        let p2_turn = self.p2_turn();

        let p1_next = p1.unwrap_or(false);
        let p2_next = !p1.unwrap_or(true);

        if p1_turn && !p1_next {
            self.p1.turn_end();
        }
        if p2_turn && !p2_next {
            self.p2.turn_end();
        }

        if !p1_turn && p1_next {
            self.p1.turn_start();
        }
        if !p2_turn && p2_next {
            self.p2.turn_start();
        }

        self.p1_turn = p1;
    }

    /// Mark that it's the first player's turn, starting their clock if
    /// necessary. This is a no-op if it's already their turn. If it's the
    /// second player's turn, their clock is stopped.
    pub fn set_p1_turn(&mut self) {
        self.set_turn(Some(true));
    }

    /// Mark that it's the second player's turn, starting their clock if
    /// necessary. This is a no-op if it's already their turn. If it's the first
    /// player's turn, their clock is stopped.
    pub fn set_p2_turn(&mut self) {
        self.set_turn(Some(false));
    }

    /// Mark that it's neither player's turn. If it's currently a player's turn,
    /// their clock is stopped.
    pub fn clear_turn(&mut self) {
        self.set_turn(None);
    }

    /// Update the clocks given the amount of time that has passed since the
    /// last update.
    pub fn turn_spend(&mut self, elapsed: Duration) {
        match self.p1_turn {
            Some(true) => self.p1.turn_spend(elapsed),
            Some(false) => self.p2.turn_spend(elapsed),
            None => false,
        };
    }
}

// Miscellaneous helpers
// -----------------------------------------------------------------------------

/// Shows `M:SS` for durations over 10 seconds, and `S.Ss` for shorter durations.
/// Durations are rounded up to the precision of the indicated format, so e.g.
/// a duration of 16,250 milliseconds would show 0:17, and for 6,250 ms would
/// show "6.3s"
struct ShowCountdown {
    show_minutes: bool,
    time: Duration,
}

impl ShowCountdown {
    fn with_minutes(time: Duration) -> Self {
        Self {
            show_minutes: true,
            time,
        }
    }

    fn without_minutes(time: Duration) -> Self {
        Self {
            show_minutes: false,
            time,
        }
    }
}

impl fmt::Display for ShowCountdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw_s = self.time.as_secs();
        let raw_ns = self.time.subsec_nanos() as u64;

        let raw_d = raw_s * 10 + raw_ns / 100_000_000;
        let raw_d_ns = raw_ns % 100_000_000;

        let s_ceil = if raw_ns > 0 { raw_s + 1 } else { raw_s };
        let d_ceil = if raw_d_ns > 0 { raw_d + 1 } else { raw_d };

        if d_ceil > 90 {
            if self.show_minutes {
                let m = s_ceil / 60;
                let s = s_ceil % 60;
                write!(f, "{m}:{s:02}")
            } else {
                write!(f, "{s_ceil}s")
            }
        } else {
            let s = d_ceil / 10;
            let d = d_ceil % 10;
            write!(f, "{s}.{d}s")
        }
    }
}

#[test]
fn test_show_countdown_with_minutes() {
    let t = |n: u64| ShowCountdown::with_minutes(Duration::from_millis(n)).to_string();

    assert_eq!(t(90_000), "1:30");
    assert_eq!(t(89_100), "1:30");
    assert_eq!(t(61_000), "1:01");
    assert_eq!(t(59_999), "1:00");
    assert_eq!(t(59_000), "0:59");

    assert_eq!(t(10_000), "0:10");
    assert_eq!(t(9_900), "0:10");
    assert_eq!(t(9_800), "0:10");
    assert_eq!(t(9_200), "0:10");
    assert_eq!(t(9_100), "0:10");
    assert_eq!(t(9_050), "0:10");
    assert_eq!(t(9_000), "9.0s");
    assert_eq!(t(8_950), "9.0s");
    assert_eq!(t(8_900), "8.9s");
    assert_eq!(t(1_110), "1.2s");
    assert_eq!(t(0), "0.0s");
}

#[test]
fn test_show_countdown_without_minutes() {
    let t = |n: u64| ShowCountdown::without_minutes(Duration::from_millis(n)).to_string();

    assert_eq!(t(90_000), "90s");
    assert_eq!(t(89_100), "90s");
    assert_eq!(t(61_000), "61s");
    assert_eq!(t(59_999), "60s");
    assert_eq!(t(59_000), "59s");

    assert_eq!(t(10_000), "10s");
    assert_eq!(t(9_900), "10s");
    assert_eq!(t(9_800), "10s");
    assert_eq!(t(9_200), "10s");
    assert_eq!(t(9_100), "10s");
    assert_eq!(t(9_050), "10s");
    assert_eq!(t(9_000), "9.0s");
    assert_eq!(t(8_950), "9.0s");
    assert_eq!(t(8_900), "8.9s");
    assert_eq!(t(1_110), "1.2s");
    assert_eq!(t(0), "0.0s");
}
