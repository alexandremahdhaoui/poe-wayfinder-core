//! The parser stages both games share.
//!
//! 35 of the 50 stages in the reference are identical between PoE1 and PoE2.
//! They live here. The game specific ones live in `poe1` and `poe2`.

pub mod combat;
pub mod flags;
pub mod levels;
pub mod misc;
