pub mod placeholder;
pub mod resolve;

pub use placeholder::{candidates, split_unscalable, StatString};
pub use resolve::try_parse_translation;
