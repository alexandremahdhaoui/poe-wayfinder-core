//! Business logic. Parsing, filtering and pricing.

pub mod aggregate;
pub mod calc;
pub mod filter;
pub mod mod_desc;
pub mod parse;
pub mod price_check;
pub mod stat_match;

pub use price_check::{price_check, PriceCheck, PriceCheckOptions};
