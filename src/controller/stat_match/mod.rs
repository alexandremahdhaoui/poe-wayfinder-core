//! Turning a stat line into a trade stat id.
//!
//! The game prints text. The trade site wants an id. Nothing in the text says
//! which numbers are rolls and which are part of the stat's name, so the
//! matcher produces every reading the line could have and takes the first the
//! data file recognises.

pub mod placeholder;
pub mod resolve;

pub use placeholder::{candidates, split_unscalable, StatString};
pub use resolve::try_parse_translation;
