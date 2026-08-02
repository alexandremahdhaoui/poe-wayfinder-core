//! Internal models. Plain data only. No behaviour beyond small helpers.

pub mod game;
pub mod modifier;

pub use game::GameVersion;
pub use modifier::{Generation, ModifierInfo, ModifierType};
