//! Turning a parsed item into a trade query.
//!
//! One rule governs everything here. Filter on what identifies the item, not
//! on everything it has. An over specified query returns nothing, and an empty
//! result reads as "this item is worthless" when it means "nobody else has
//! this exact roll".

pub mod augments;
pub mod bounds;
pub mod brackets;
pub mod common;
pub mod item_filters;
pub mod item_property;
pub mod presets;
pub mod pseudo;
pub mod roll_math;
pub mod rules;
pub mod slots;
pub mod stat_filters;
pub mod unique;

pub use item_filters::{build_query, FilterOptions};
pub use pseudo::{pseudo_totals, PseudoTotal};
pub use stat_filters::{build_stat_group, StatFilterOptions};
