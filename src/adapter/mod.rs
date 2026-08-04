//! What the domain needs from the outside world.
//!
//! This module declares traits and implements none of them. The parser needs
//! game data and must not read a file, so it states the need here and
//! `poe-trader-app` provides it.

pub mod data_adapter;

pub use data_adapter::{GameData, ItemLookup, Namespace, StatLookup};
