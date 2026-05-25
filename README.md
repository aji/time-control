# A game clock library for Rust

This crate provides implementations of a few common time control systems for
board games. Currently it includes the following:

- `SimpleDelayClock`, which on each turn waits for a fixed period before the
main clock starts counting down.

- `FischerClock`, which adds a fixed amount of time to the clock at the end
of the turn.

- `ByoYomiClock`, where after the main clock counts down the player has one
or more "byo-yomi periods" which are reset at the end of the turn if not
fully consumed.

All accounting is done by processing `Duration` values which represent the time
since the last update, allowing flexible timekeeping.