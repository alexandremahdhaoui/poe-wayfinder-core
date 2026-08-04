//! Internal models. Plain data only. No behaviour beyond small helpers.

pub mod category;
pub mod client_strings;
pub mod game;
pub mod item;
pub mod modifier;
pub mod stat;

pub use category::ItemCategory;
pub use game::GameVersion;
pub use item::{Influence, ItemRarity, ParsedItem};
pub use modifier::{Generation, ModifierInfo, ModifierType};
pub use stat::{ParsedStat, Stat, StatBetter, StatMatcher, StatRoll};
