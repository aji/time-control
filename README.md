# A game clock library for Rust

This is a small crate with implementations of various common time controls for
turn-based games like chess and go. All accounting is done by processing
`Duration` values which represent the time since the last update, allowing
flexible timekeeping.

The following clocks are included:

- [`SimpleDelayClock`] and [`SimpleDelayConfig`], which at the start of a
turn waits for a delay period before counting down the main time.

- [`FischerClock`] and [`FischerConfig`], which adds a fixed increment at
the end of a turn.

- [`BronsteinClock`] and [`BronsteinConfig`], which returns the time spent
during the turn, up to a certain limit.

- [`ByoYomiClock`] and [`ByoYomiConfig`], which after the main time has
expired begins counting down byo-yomi periods which reset at the end of the
turn if not fully consumed.

Additionally, it includes [`AnyClock`] and [`AnyConfig`] which are
enumerations of the above, allowing generic handling of clocks and
configurations. There is also a `dyn`-compatible [`TimeControl`] trait
that can be used to a similar effect.

Finally, [`TwoPlayer`] provides convenience functionality for managing
clocks for two players, where the currently decrementing clock alternates
between players.

# Example

Here's an example of using the [`TimeControl`] API directly for a single
clock:

```rust
# use time_control::*;
# use std::time::Duration;
let _1sec = Duration::from_secs(1);
let _1min = Duration::from_mins(1);

// Our clock config, a simple 3:00 clock with 5s increment.
let cf = FischerConfig::new(3, 5);

// The clock itself:
let mut clk = FischerClock::new(cf);

// Initially, the clock shows 3:00
assert_eq!(clk.to_string(), "3:00");

// We can start a turn, then spend some time as it counts down:
clk.turn_start();
clk.turn_spend(_1sec);
assert_eq!(clk.to_string(), "2:59");
clk.turn_spend(_1sec);
assert_eq!(clk.to_string(), "2:58");
clk.turn_spend(_1sec);
assert_eq!(clk.to_string(), "2:57");

// When we end our turn, the 5s increment is added:
clk.turn_end();
assert_eq!(clk.to_string(), "3:02");

// Now let's start our next turn and spend too much time thinking...
clk.turn_start();
assert_eq!(clk.turn_spend(_1min), false);
assert_eq!(clk.to_string(), "2:02");
assert_eq!(clk.turn_spend(_1min), false);
assert_eq!(clk.to_string(), "1:02");
assert_eq!(clk.turn_spend(_1min), false);
assert_eq!(clk.to_string(), "2.0s");
assert_eq!(clk.turn_spend(_1sec), false);
assert_eq!(clk.to_string(), "1.0s");

// After 1 more second, the clock expires!
assert_eq!(clk.turn_spend(_1sec), true);
assert_eq!(clk.to_string(), "0.0s");
assert!(clk.is_expired());
clk.turn_end();
assert_eq!(clk.to_string(), "0.0s");
```