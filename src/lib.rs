#![cfg_attr(not(test), no_std)]

//! A small crate with implementations of various time controls for turn-based
//! games like chess and go. It includes the following clocks:
//!
//! - [`SimpleDelayClock`] and [`SimpleDelayConfig`], which at the start of a
//! turn waits for a delay period before counting down the main time.
//!
//! - [`FischerClock`] and [`FischerConfig`], which adds a fixed increment at
//! the end of a turn.
//!
//! - [`BronsteinClock`] and [`BronsteinConfig`], which returns the time spent
//! during the turn, up to a certain limit.
//!
//! - [`ByoYomiClock`] and [`ByoYomiConfig`], which after the main time has
//! expired begins counting down byo-yomi periods which reset at the end of the
//! turn if not fully consumed.
//!
//! Additionally, it includes [`AnyClock`] and [`AnyConfig`] which are
//! enumerations of the above, allowing generic handling of clocks and
//! configurations. There is also a `dyn`-compatible [`TimeControl`] trait
//! that can be used to a similar effect.
//!
//! Finally, [`TwoPlayer`] provides convenience functionality for managing
//! clocks for two players, where the currently decrementing clock alternates
//! between players.
//!
//! # Example
//!
//! Here's an example of using the [`TimeControl`] API directly for a single
//! clock:
//!
//! ```rust
//! # use time_control::*;
//! # use std::time::Duration;
//! let _1sec = Duration::from_secs(1);
//! let _1min = Duration::from_mins(1);
//!
//! // Our clock config, a simple 3:00 clock with 5s increment.
//! let cf = FischerConfig::new(3, 5);
//!
//! // The clock itself:
//! let mut clk = FischerClock::new(cf);
//!
//! // Initially, the clock shows 3:00
//! assert_eq!(clk.to_string(), "3:00");
//!
//! // We can start a turn, then spend some time as it counts down:
//! clk.turn_start();
//! clk.turn_spend(_1sec);
//! assert_eq!(clk.to_string(), "2:59");
//! clk.turn_spend(_1sec);
//! assert_eq!(clk.to_string(), "2:58");
//! clk.turn_spend(_1sec);
//! assert_eq!(clk.to_string(), "2:57");
//!
//! // When we end our turn, the 5s increment is added:
//! clk.turn_end();
//! assert_eq!(clk.to_string(), "3:02");
//!
//! // Now let's start our next turn and spend too much time thinking...
//! clk.turn_start();
//! assert_eq!(clk.turn_spend(_1min), false);
//! assert_eq!(clk.to_string(), "2:02");
//! assert_eq!(clk.turn_spend(_1min), false);
//! assert_eq!(clk.to_string(), "1:02");
//! assert_eq!(clk.turn_spend(_1min), false);
//! assert_eq!(clk.to_string(), "2.0s");
//! assert_eq!(clk.turn_spend(_1sec), false);
//! assert_eq!(clk.to_string(), "1.0s");
//!
//! // After 1 more second, the clock expires!
//! assert_eq!(clk.turn_spend(_1sec), true);
//! assert_eq!(clk.to_string(), "0.0s");
//! assert!(clk.is_expired());
//! clk.turn_end();
//! assert_eq!(clk.to_string(), "0.0s");
//! ```

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
///    during the turn, and the return value indicates if the clock expired.
/// - `turn_end` at the end of the turn.
///
/// When a clock expires, it remains in the expired state until it is reset.
pub trait TimeControl {
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
    /// clock is already expired, this cannot un-expire it. This is a no-op if
    /// `turn_end` has not been called since the last call to `turn_start`
    fn turn_start(&mut self);

    /// Record that this player has spent the given amount of time thinking.
    /// Returns `true` if the clock has expired, or `false` if there is still
    /// time remaining. This is a no-op if `turn_end` has been called since the
    /// last call to `turn_start`.
    fn turn_spend(&mut self, elapsed: Duration) -> bool;

    /// Apply any clock adjustments for the end of this player's turn. If the
    /// clock is already expired, this cannot un-expire it. This is a no-op if
    /// `turn_end` has already been called since the clast call to `turn_start`.
    fn turn_end(&mut self);

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

impl SimpleDelayConfig {
    /// Create a new simple delay clock configuration with the given initial
    /// minutes and delay seconds.
    pub fn new(initial_mins: u64, delay_secs: u64) -> SimpleDelayConfig {
        SimpleDelayConfig {
            initial: Duration::from_mins(initial_mins),
            delay: Duration::from_secs(delay_secs),
        }
    }
}

/// A simple delay clock
///
/// See [`SimpleDelayConfig`] for more information.
#[derive(Copy, Clone, Debug)]
pub struct SimpleDelayClock {
    config: SimpleDelayConfig,
    running: bool,
    delay: Duration,
    main: Duration,
}

impl SimpleDelayClock {
    /// Create a new simple delay clock with the given config.
    pub fn new(config: SimpleDelayConfig) -> SimpleDelayClock {
        SimpleDelayClock {
            config,
            running: false,
            delay: Duration::ZERO,
            main: config.initial,
        }
    }

    /// Return a copy of the config used to create this clock
    pub fn config(&self) -> SimpleDelayConfig {
        self.config
    }

    /// Return the time remaining on the clock, not accounting for the delay
    pub fn main_remaining(&self) -> Duration {
        self.main
    }

    /// Return the delay period time remaining
    pub fn delay_remaining(&self) -> Duration {
        self.delay
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
        if !self.running {
            self.running = true;
            self.delay = match self.is_expired() {
                true => Duration::ZERO,
                false => self.config.delay,
            };
        }
    }

    fn turn_spend(&mut self, elapsed: Duration) -> bool {
        if !self.running {
            return self.main.is_zero();
        }

        let delay_use = self.delay.min(elapsed);
        let main_use = elapsed - delay_use;

        self.delay = self.delay.saturating_sub(delay_use);
        self.main = self.main.saturating_sub(main_use);

        self.main.is_zero()
    }

    fn turn_end(&mut self) {
        if self.running {
            self.running = false;
            self.delay = Duration::ZERO;
        }
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

#[test]
fn test_simple_delay() {
    let t = Duration::from_secs;

    let mut clk = SimpleDelayClock::new(SimpleDelayConfig::new(1, 10));

    let spend = |clk: &mut SimpleDelayClock, secs| -> (bool, u64, u64) {
        let expired = clk.turn_spend(t(secs));
        (
            expired,
            clk.main_remaining().as_secs(),
            clk.delay_remaining().as_secs(),
        )
    };

    clk.turn_start();
    assert_eq!(spend(&mut clk, 0), (false, 60, 10));
    assert_eq!(spend(&mut clk, 5), (false, 60, 5));
    assert_eq!(spend(&mut clk, 5), (false, 60, 0));
    assert_eq!(spend(&mut clk, 5), (false, 55, 0));
    assert_eq!(spend(&mut clk, 15), (false, 40, 0));
    assert_eq!(spend(&mut clk, 20), (false, 20, 0));
    assert_eq!(spend(&mut clk, 20), (true, 0, 0));
    assert_eq!(spend(&mut clk, 20), (true, 0, 0));
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

    /// A limit to the amount of time on the clock after incrementing.
    pub limit: Option<Duration>,
}

impl Into<FischerClock> for FischerConfig {
    fn into(self) -> FischerClock {
        FischerClock::new(self)
    }
}

impl FischerConfig {
    /// Create a new Fischer clock config with the given initial minutes and
    /// increment seconds.
    pub fn new(initial_mins: u64, increment_secs: u64) -> FischerConfig {
        FischerConfig {
            initial: Duration::from_mins(initial_mins),
            increment: Duration::from_secs(increment_secs),
            limit: None,
        }
    }
}

/// A Fischer clock
///
/// See [`FischerConfig`] for more details.
#[derive(Copy, Clone, Debug)]
pub struct FischerClock {
    config: FischerConfig,
    running: bool,
    main: Duration,
}

impl FischerClock {
    /// Create a new Fischer clock with the given config.
    pub fn new(config: FischerConfig) -> FischerClock {
        FischerClock {
            config,
            running: false,
            main: config.initial,
        }
    }

    /// Return a copy of the config used to create this clock
    pub fn config(&self) -> FischerConfig {
        self.config
    }

    /// Return the time remaining on the clock
    pub fn main_remaining(&self) -> Duration {
        self.main
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

    fn turn_start(&mut self) {
        self.running = true;
    }

    fn turn_spend(&mut self, elapsed: Duration) -> bool {
        if self.running {
            self.main = self.main.saturating_sub(elapsed);
        }
        self.main.is_zero()
    }

    fn turn_end(&mut self) {
        if self.running {
            self.running = false;
            if !self.main.is_zero() {
                self.main += self.config.increment;
                if let Some(limit) = self.config.limit {
                    self.main = self.main.min(limit);
                }
            }
        }
    }
}

impl fmt::Display for FischerClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", ShowCountdown::with_minutes(self.main))
    }
}

#[test]
fn test_fischer() {
    let t = Duration::from_secs;

    let mut clk = FischerClock::new(FischerConfig::new(1, 10));

    let spend = |clk: &mut FischerClock, secs| -> (bool, u64) {
        let expired = clk.turn_spend(t(secs));
        (expired, clk.main_remaining().as_secs())
    };

    clk.turn_start();
    assert_eq!(spend(&mut clk, 0), (false, 60));
    assert_eq!(spend(&mut clk, 5), (false, 55));
    assert_eq!(spend(&mut clk, 15), (false, 40));
    assert_eq!(spend(&mut clk, 20), (false, 20));
    clk.turn_end();
    assert_eq!(clk.main_remaining(), t(30));
    clk.turn_start();
    assert_eq!(spend(&mut clk, 20), (false, 10));
    assert_eq!(spend(&mut clk, 20), (true, 0));

    let mut clk = FischerClock::new(FischerConfig {
        initial: t(60),
        increment: t(10),
        limit: Some(t(70)),
    });

    clk.turn_start();
    assert_eq!(spend(&mut clk, 5), (false, 55));
    clk.turn_end();
    clk.turn_start();
    assert_eq!(spend(&mut clk, 5), (false, 60));
    clk.turn_end();
    clk.turn_start();
    assert_eq!(spend(&mut clk, 5), (false, 65));
    clk.turn_end();
    clk.turn_start();
    assert_eq!(spend(&mut clk, 5), (false, 65));
    clk.turn_end();
}

// Bronstein delay
// -----------------------------------------------------------------------------

/// Configuration for a clock with Bronstein delay rules
///
/// This clock is like Fischer but at the end of a turn adds no more time than
/// was used during the move. For example, if the increment is 10 seconds and
/// a player only uses 5 seconds, then only 5 seconds are added to their clock
/// at the end of their turn. If they use 20 seconds, then only 10 seconds are
/// added to their clock.
///
/// This type of clock is very similar to the simple delay clock, with the
/// difference essentially being that the delay is accounted for at the end of
/// the turn rather than the start.
#[derive(Copy, Clone, Debug)]
pub struct BronsteinConfig {
    /// The amount of time on the clock at the start of the game.
    pub initial: Duration,

    /// The maximum amount of time added to the clock at the end of the turn.
    pub max_increment: Duration,
}

impl Into<BronsteinClock> for BronsteinConfig {
    fn into(self) -> BronsteinClock {
        BronsteinClock::new(self)
    }
}

impl BronsteinConfig {
    /// Create a new Bronstein clock config with the given initial minutes and
    /// max increment seconds.
    pub fn new(initial_mins: u64, max_increment_secs: u64) -> BronsteinConfig {
        BronsteinConfig {
            initial: Duration::from_mins(initial_mins),
            max_increment: Duration::from_secs(max_increment_secs),
        }
    }
}

/// A Bronstein delay clock
///
/// See [`BronsteinConfig`] for more information.
#[derive(Copy, Clone, Debug)]
pub struct BronsteinClock {
    config: BronsteinConfig,
    running: bool,
    main: Duration,
    spent: Duration,
}

impl BronsteinClock {
    /// Create a new Bronstein clock with the given config.
    pub fn new(config: BronsteinConfig) -> BronsteinClock {
        BronsteinClock {
            config,
            running: false,
            main: config.initial,
            spent: Duration::ZERO,
        }
    }

    /// Return a copy of the config used to create this clock
    pub fn config(&self) -> BronsteinConfig {
        self.config
    }

    /// Return the time remaining on the clock
    pub fn main_remaining(&self) -> Duration {
        self.main
    }
}

impl TimeControl for BronsteinClock {
    fn reset(&mut self) {
        *self = Self::new(self.config)
    }

    fn is_expired(&self) -> bool {
        self.main.is_zero()
    }

    fn max_remaining(&self) -> Duration {
        self.main
    }

    fn turn_start(&mut self) {
        self.running = true;
    }

    fn turn_spend(&mut self, elapsed: Duration) -> bool {
        if self.running {
            self.spent += elapsed;
            self.main = self.main.saturating_sub(elapsed);
        }
        self.main.is_zero()
    }

    fn turn_end(&mut self) {
        if self.running {
            self.running = false;
            if !self.main.is_zero() {
                self.main += self.config.max_increment.min(self.spent);
            }
            self.spent = Duration::ZERO;
        }
    }
}

impl fmt::Display for BronsteinClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", ShowCountdown::with_minutes(self.main))
    }
}

#[test]
fn test_bronstein() {
    let t = Duration::from_secs;

    let mut clk = BronsteinClock::new(BronsteinConfig::new(1, 10));

    let spend = |clk: &mut BronsteinClock, secs| -> (bool, u64) {
        let expired = clk.turn_spend(t(secs));
        (expired, clk.main_remaining().as_secs())
    };

    assert_eq!(clk.main_remaining(), t(60));
    clk.turn_start();
    assert_eq!(spend(&mut clk, 5), (false, 55));
    clk.turn_end();
    assert_eq!(clk.main_remaining(), t(60));
    clk.turn_start();
    assert_eq!(spend(&mut clk, 20), (false, 40));
    clk.turn_end();
    assert_eq!(clk.main_remaining(), t(50));
    clk.turn_start();
    assert_eq!(spend(&mut clk, 5), (false, 45));
    clk.turn_end();
    assert_eq!(clk.main_remaining(), t(50));
    clk.turn_start();
    assert_eq!(spend(&mut clk, 20), (false, 30));
    assert_eq!(spend(&mut clk, 20), (false, 10));
    assert_eq!(spend(&mut clk, 20), (true, 0));
    clk.turn_end();
    assert_eq!(clk.main_remaining(), t(0));
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

impl ByoYomiConfig {
    /// Create a new byo-yomi clock config with the given initial minutes,
    /// period count, and period seconds.
    pub fn new(initial_mins: u64, num_periods: usize, period_secs: u64) -> ByoYomiConfig {
        ByoYomiConfig {
            initial: Duration::from_mins(initial_mins),
            period_time: Duration::from_secs(period_secs),
            num_periods,
        }
    }
}

/// A byo-yomi clock
///
/// See [`ByoYomiConfig`] for more details.
#[derive(Copy, Clone, Debug)]
pub struct ByoYomiClock {
    config: ByoYomiConfig,
    running: bool,
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
            running: false,
            main: config.initial,
            period: Duration::ZERO,
            unused_periods: config.num_periods,
        }
    }

    /// Return a copy of the config used to create this clock
    pub fn config(&self) -> ByoYomiConfig {
        self.config
    }

    /// Return the time remaining on the main countdown. This is zero when
    /// counting down byo-yomi periods.
    pub fn main_remaining(&self) -> Duration {
        self.main
    }

    /// Return the number of byo-yomi periods remaining on the clock, including
    /// the period currently counting down, if applicable
    pub fn periods_remaining(&self) -> usize {
        self.unused_periods + (!self.period.is_zero()) as usize
    }

    /// Return the amount of time remaining for the current period. This will be
    /// zero during byo-yomi between `turn_end` and `turn_start`
    pub fn this_period_remaining(&self) -> Duration {
        self.period
    }

    /// Return the amount of time remaining for all byo-yomi periods. This plus
    /// `main_remaining` is the total amount of that can be spent on the current
    /// turn before the clock expires.
    pub fn all_periods_remaining(&self) -> Duration {
        self.period + self.config.period_time * self.unused_periods as u32
    }

    /// Return whether this clock is counting down byo-yomi periods
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
        self.running = true;
        if self.in_byo_yomi() && self.period.is_zero() && self.unused_periods > 0 {
            self.unused_periods -= 1;
            self.period = self.config.period_time;
        }
    }

    fn turn_spend(&mut self, mut elapsed: Duration) -> bool {
        if !self.running {
            return self.is_expired();
        }

        // spend main time. this is a no-op if our main time is zero
        let spend_main = elapsed.min(self.main);
        self.main -= spend_main;
        elapsed -= spend_main;

        // start a byo-yomi period if necessary
        if self.in_byo_yomi() && self.period.is_zero() && self.unused_periods > 0 {
            self.period = self.config.period_time;
            self.unused_periods -= 1;
        }

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
        if self.running {
            self.running = false;
            if self.in_byo_yomi() && !self.period.is_zero() {
                // return the unspent byo-yomi period.
                self.period = Duration::ZERO;
                self.unused_periods += 1;
            }
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

#[test]
fn test_byo_yomi() {
    let t = Duration::from_secs;

    // 1:00 +3x10
    let mut clk = ByoYomiClock::new(ByoYomiConfig::new(1, 3, 10));

    let info = |clk: &ByoYomiClock| -> (u64, u64, usize) {
        (
            clk.main_remaining().as_secs(),
            clk.this_period_remaining().as_secs(),
            clk.unused_periods,
        )
    };
    let spend = |clk: &mut ByoYomiClock, secs| -> (bool, u64, u64, usize) {
        let expired = clk.turn_spend(t(secs));
        (
            expired,
            clk.main_remaining().as_secs(),
            clk.this_period_remaining().as_secs(),
            clk.unused_periods,
        )
    };

    assert_eq!(info(&clk), (60, 0, 3));
    clk.turn_start();
    assert_eq!(info(&clk), (60, 0, 3));
    assert_eq!(spend(&mut clk, 20), (false, 40, 0, 3));
    assert_eq!(spend(&mut clk, 20), (false, 20, 0, 3));
    assert_eq!(spend(&mut clk, 20), (false, 0, 10, 2));
    assert_eq!(spend(&mut clk, 5), (false, 0, 5, 2));
    assert_eq!(spend(&mut clk, 5), (false, 0, 10, 1));
    assert_eq!(spend(&mut clk, 5), (false, 0, 5, 1));
    clk.turn_end();
    assert_eq!(info(&clk), (0, 0, 2));
    clk.turn_start();
    assert_eq!(info(&clk), (0, 10, 1));
    assert_eq!(spend(&mut clk, 10), (false, 0, 10, 0));
    assert_eq!(spend(&mut clk, 10), (true, 0, 0, 0));
    assert_eq!(spend(&mut clk, 10), (true, 0, 0, 0));
}

// AnyClock
// -----------------------------------------------------------------------------

macro_rules! any_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($variant:ident($ty:ty),)*
        }
    ) => {
        $(#[$meta])*
        pub enum $name {
            $($variant($ty),)*
        }

        $(
            impl From<$ty> for $name {
                fn from(value: $ty) -> Self {
                    Self::$variant(value)
                }
            }

            impl TryFrom<$name> for $ty {
                type Error = $name;

                fn try_from(value: $name) -> Result<Self, Self::Error> {
                    match value {
                        $name::$variant(inner) => Ok(inner),
                        value => Err(value),
                    }
                }
            }
        )*
    };
}

any_enum!(
    /// An enum over all clock configuration types in this crate
    #[derive(Copy, Clone, Debug)]
    pub enum AnyConfig {
        SimpleDelay(SimpleDelayConfig),
        Fischer(FischerConfig),
        Bronstein(BronsteinConfig),
        ByoYomi(ByoYomiConfig),
    }
);

any_enum!(
    /// An enum over all clock types in this crate
    #[derive(Copy, Clone, Debug)]
    pub enum AnyClock {
        SimpleDelay(SimpleDelayClock),
        Fischer(FischerClock),
        Bronstein(BronsteinClock),
        ByoYomi(ByoYomiClock),
    }
);

impl<T> From<T> for AnyClock
where
    T: Into<AnyConfig>,
{
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl AnyClock {
    pub fn new<T: Into<AnyConfig>>(config: T) -> AnyClock {
        match config.into() {
            AnyConfig::SimpleDelay(config) => AnyClock::SimpleDelay(SimpleDelayClock::new(config)),
            AnyConfig::Fischer(config) => AnyClock::Fischer(FischerClock::new(config)),
            AnyConfig::Bronstein(config) => AnyClock::Bronstein(BronsteinClock::new(config)),
            AnyConfig::ByoYomi(config) => AnyClock::ByoYomi(ByoYomiClock::new(config)),
        }
    }
}

macro_rules! any_clock_proxy {
    (fn $name:ident(&self $(, $arg:ident : $argty:ty)*) -> $ret:ty) => {
        fn $name(&self, $($arg: $argty)*) -> $ret {
            match self {
                AnyClock::SimpleDelay(clock) => clock.$name($($arg),*),
                AnyClock::Fischer(clock) => clock.$name($($arg),*),
                AnyClock::Bronstein(clock) => clock.$name($($arg),*),
                AnyClock::ByoYomi(clock) => clock.$name($($arg),*),
            }
        }
    };

    (fn $name:ident(&mut self $(, $arg:ident : $argty:ty)*) -> $ret:ty) => {
        fn $name(&mut self, $($arg: $argty)*) -> $ret {
            match self {
                AnyClock::SimpleDelay(clock) => clock.$name($($arg),*),
                AnyClock::Fischer(clock) => clock.$name($($arg),*),
                AnyClock::Bronstein(clock) => clock.$name($($arg),*),
                AnyClock::ByoYomi(clock) => clock.$name($($arg),*),
            }
        }
    };
}

impl fmt::Display for AnyClock {
    any_clock_proxy!(fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result);
}

impl TimeControl for AnyClock {
    any_clock_proxy!(fn reset(&mut self) -> ());
    any_clock_proxy!(fn is_expired(&self) -> bool);
    any_clock_proxy!(fn max_remaining(&self) -> Duration);
    any_clock_proxy!(fn turn_start(&mut self) -> ());
    any_clock_proxy!(fn turn_spend(&mut self, elapsed: Duration) -> bool);
    any_clock_proxy!(fn turn_end(&mut self) -> ());
    any_clock_proxy!(fn complete_turn(&mut self, elapsed: Duration) -> bool);
}

// TwoPlayer
// -----------------------------------------------------------------------------

/// Track time controls for two players
///
/// Time is spent with [`turn_spend`][`Self::turn_spend`], and the player whose
/// clock this is accounted to is changed with [`set_p1_turn`][`Self::set_p1_turn`]
/// and [`set_p2_turn`][`Self::set_p2_turn`]. When either player's clock expires,
/// further calls to [`turn_spend`][`Self::turn_spend`] have no effect.
///
/// # Example
///
/// ```
/// # use time_control::*;
/// # use std::time::Duration;
/// let _1sec = Duration::from_secs(1);
/// let _1min = Duration::from_mins(1);
///
/// let cf = FischerConfig::new(3, 5);
/// let mut tc: TwoPlayer<FischerClock> = TwoPlayer::new(cf);
///
/// let show = |tc: &TwoPlayer<FischerClock>| format!("{} {}", tc.p1(), tc.p2());
///
/// assert_eq!(show(&tc), "3:00 3:00");
///
/// tc.set_p1_turn();
/// tc.turn_spend(_1min);
/// assert_eq!(show(&tc), "2:00 3:00");
///
/// tc.set_p2_turn();
/// tc.turn_spend(_1min);
/// tc.turn_spend(_1min);
/// assert_eq!(show(&tc), "2:05 1:00");
///
/// tc.set_p1_turn();
/// tc.turn_spend(_1min);
/// tc.turn_spend(_1min);
/// assert_eq!(show(&tc), "5.0s 1:05");
///
/// tc.set_p2_turn();
/// tc.turn_spend(_1min);
/// assert_eq!(show(&tc), "0:10 5.0s");
/// assert!(!tc.turn_spend(_1sec));
/// assert!(!tc.turn_spend(_1sec));
/// assert!(!tc.turn_spend(_1sec));
/// assert_eq!(show(&tc), "0:10 2.0s");
/// assert!(!tc.turn_spend(_1sec));
/// assert_eq!(show(&tc), "0:10 1.0s");
///
/// // now it expires:
/// assert!(tc.turn_spend(_1sec));
/// assert_eq!(show(&tc), "0:10 0.0s");
/// ```
pub struct TwoPlayer<P1, P2 = P1> {
    p1_lose: Option<bool>,
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
            p1_lose: None,
            p1_turn: None,
            p1: p1.into(),
            p2: p2.into(),
        }
    }

    /// Reset the time control to its initial configuration. After resetting,
    /// the time control will be stopped for both players.
    pub fn reset(&mut self) {
        self.p1_lose = None;
        self.p1_turn = None;
        self.p1.reset();
        self.p2.reset();
    }

    /// Return whether either player's clock has expired
    pub fn is_expired(&self) -> bool {
        self.p1_lose.is_some()
    }

    /// Return whether the first player's clock is counting down.
    pub fn p1_turn(&self) -> bool {
        self.p1_lose.is_none() && self.p1_turn.unwrap_or(false)
    }

    /// Return whether the second player's clock is counting down.
    pub fn p2_turn(&self) -> bool {
        self.p1_lose.is_none() && !self.p1_turn.unwrap_or(true)
    }

    /// Return a reference to the first player's time control
    pub fn p1(&self) -> &P1 {
        &self.p1
    }

    /// Return a reference to the second player's time control
    pub fn p2(&self) -> &P2 {
        &self.p2
    }

    fn set_turn(&mut self, p1: Option<bool>) {
        if self.p1_lose.is_some() {
            return;
        }

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
    /// last update. Returns whether either player's clock has expired
    pub fn turn_spend(&mut self, elapsed: Duration) -> bool {
        if self.p1_lose.is_none() {
            let p1_expired = self.p1.turn_spend(elapsed);
            let p2_expired = self.p2.turn_spend(elapsed);

            if p1_expired {
                self.clear_turn();
                self.p1_lose = Some(true);
            }
            if p2_expired {
                self.clear_turn();
                self.p1_lose = Some(false);
            }

            p1_expired || p2_expired
        } else {
            true
        }
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
