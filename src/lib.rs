//! Path of Exile item parsing and trade query building.
//!
//! This crate performs no I/O. It has no network client and no window and no
//! filesystem access. It takes item text and returns a parsed item. It takes a
//! parsed item and returns a trade query.
//!
//! That constraint is what makes the parser testable without a game running.
//! The one thing it needs from the outside is game data, and it declares that
//! need as a trait in [`adapter::GameData`] rather than reading a file.

pub mod adapter;
pub mod controller;
pub mod types;
pub mod util;
